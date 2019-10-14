extern crate byteorder;
extern crate chrono;
extern crate derivative;
extern crate exif;

use super::exif::read_date_time_original_as_utc;
use byteorder::{ByteOrder, LittleEndian, ReadBytesExt};
use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::{Tz, UTC};
use derivative::Derivative;
use exif::Reader;
use std::cmp::Ordering;
use std::default::Default;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, BufReader, SeekFrom};

pub fn read_x3f_time(filename: &str, from_tz: Option<Tz>) -> Result<Option<DateTime<Tz>>, String> {
    let file = File::open(filename).map_err(|e| e.to_string())?;
    let reader = X3fReader::new(BufReader::new(file), from_tz).map_err(|e| e.to_string())?;
    Ok(reader.get_taken_datetime())
}

struct X3fReader<R: Read + Seek> {
    inner: R,
    properties: Vec<X3fProperty>,
    exif_datetime: Option<DateTime<Tz>>,
    from_tz: Option<Tz>,
}

#[derive(Debug)]
struct X3fDirectoryEntry {
    name: String,
    offset: u32,
    length: u32,
}

#[derive(Derivative)]
#[derivative(Debug)]
struct X3fImage {
    image_type: u32,
    data_format: u32,
    columns: u32,
    rows: u32,
    row_stride: u32,
    #[derivative(Debug = "ignore")]
    data: Vec<u8>,
}

#[derive(Debug)]
struct X3fPropertyEntry {
    name_offset: u32,
    value_offset: u32,
}

#[derive(Debug)]
struct X3fProperty {
    name: String,
    value: String,
}

impl X3fImage {
    pub fn is_jpeg_thumbnail(&self) -> bool {
        self.image_type == 2 /* thumbnail */ && self.data_format == 18 /* JPEG */
    }
}

impl<R: Read + Seek> X3fReader<R> {
    pub fn new(inner: R, from_tz: Option<Tz>) -> Result<Self, X3fError> {
        let mut reader = X3fReader {
            inner,
            properties: Default::default(),
            exif_datetime: None,
            from_tz,
        };
        reader.read()?;
        Ok(reader)
    }

    fn get_property(&self, name: &str) -> Option<&String> {
        // This may be an inefficient method, but it shouldn't be a problem in the regular case.
        self.properties
            .iter()
            .find(|p| p.name == name)
            .map(|p| &p.value)
    }

    fn get_taken_datetime(&self) -> Option<DateTime<Tz>> {
        // Prefer Exif::DateTimeOrigial rather than PROP::TIME.
        match self.exif_datetime {
            Some(dt) => Some(dt),
            None => match self.get_property("TIME") {
                Some(time_str) => {
                    // Since the time zone of the Sigma camera's internal clock is UTC,
                    // there may be a time difference from the user's perception.
                    let timestamp = time_str.parse::<i64>().unwrap();
                    Some(Utc.timestamp(timestamp, 0).with_timezone(&UTC))
                }
                None => None,
            },
        }
    }

    fn read(&mut self) -> Result<(), X3fError> {
        self.check_identifier()?;

        let dir_entries = self.read_directory_entries()?;
        dbg!(&dir_entries);

        for entry in dir_entries.iter() {
            let offset = entry.offset as u64;
            let length = entry.length as u64;
            match entry.name.as_str() {
                "CAMF" => { /* TODO */ }
                "IMAG" => { /* UNSUPPORTED */ }
                "IMA2" => {
                    let image = self.read_image(offset, length)?;
                    dbg!(&image);
                    if image.is_jpeg_thumbnail() {
                        if let Some(utc) = self.read_datetime_from_thumbnail(&image) {
                            self.exif_datetime = Some(utc);
                        }
                    }
                }
                "PROP" => self.properties = self.read_property_list(offset)?,
                _ => {}
            }
        }

        Ok(())
    }

    fn check_identifier(&mut self) -> Result<(), X3fError> {
        self.seek_to(0)?;

        // Verify the identifier.
        let mut buf = [0; 4];
        self.inner.read_exact(&mut buf)?;
        if buf.cmp(&b"FOVb") != Ordering::Equal {
            return Err(X3fError::Format("Not a X3F (FOVb) file".to_owned()));
        }

        // Read the version of X3F.
        let version = self.read_u32()?;
        let fovb_version_str = format!("{:08x}", version);
        dbg!(fovb_version_str);

        Ok(())
    }

    fn check_directory(&mut self) -> Result<u32, X3fError> {
        // Read the offset of the directory section and go there.
        let dir_offset = self.read_directory_offset()?;
        dbg!(dir_offset);
        self.seek_to(dir_offset)?;

        // Verify the section identifier.
        let mut buf = [0; 4];
        self.inner.read_exact(&mut buf)?;
        if buf.cmp(&b"SECd") != Ordering::Equal {
            return Err(X3fError::Format("SECd not found".to_owned()));
        }

        // Verify the section version.
        let version = self.read_u32()?;
        let secd_version_str = format!("{:08x}", version);
        dbg!(secd_version_str);
        if version != 0x20000 {
            return Err(X3fError::Format("Unsupported SECd version".to_owned()));
        }

        // Read the number of the directory entries.
        let num_entries = self.read_u32()?;
        Ok(num_entries)
    }

    fn read_directory_entries(&mut self) -> Result<Vec<X3fDirectoryEntry>, X3fError> {
        let num_directory_entries = self.check_directory()?;
        dbg!(num_directory_entries);

        let mut entries = Vec::new();
        for _ in 0..num_directory_entries {
            let offset = self.read_u32()?;
            let length = self.read_u32()?;
            let mut buf = [0; 4];
            self.inner.read_exact(&mut buf)?;
            let name = String::from_utf8_lossy(&buf); // Cow
            let entry = X3fDirectoryEntry {
                name: String::from(name),
                offset,
                length,
            };
            entries.push(entry);
        }
        Ok(entries)
    }

    fn read_directory_offset(&mut self) -> Result<u64, io::Error> {
        self.inner.seek(SeekFrom::End(-4))?;
        let offset = self.read_u32()?;
        Ok(offset as u64)
    }

    fn read_datetime_from_thumbnail(&self, image: &X3fImage) -> Option<DateTime<Tz>> {
        match Reader::new(&mut BufReader::new(image.data.as_slice())) {
            Ok(reader) => read_date_time_original_as_utc(&reader, self.from_tz),
            Err(e) => { dbg!(e); None }
        }
    }

    fn read_image(&mut self, offset: u64, length: u64) -> Result<X3fImage, X3fError> {
        const IMAGE_HEADER_SIZE: usize = 28;

        // Go to the image section.
        self.seek_to(offset)?;

        // Verify the section identifiers.
        let mut buf = [0; 4];
        self.inner.read_exact(&mut buf)?;
        if buf.cmp(&b"SECi") != Ordering::Equal {
            return Err(X3fError::Format("SECi not found".to_owned()));
        }

        // Verify the section version.
        let version = self.read_u32()?;
        let seci_version_str = format!("{:08x}", version);
        dbg!(seci_version_str);
        if version != 0x20000 {
            return Err(X3fError::Format("Unsupported SECi version".to_owned()));
        }

        // Read the image properties.
        let image_type = self.read_u32()?;
        let data_format = self.read_u32()?;
        let columns = self.read_u32()?;
        let rows = self.read_u32()?;
        let row_stride = self.read_u32()?;

        let mut image_data = X3fImage {
            image_type,
            data_format,
            columns,
            rows,
            row_stride,
            data: Default::default(),
        };

        // Read the image data if it is a JPEG thumbnail.
        if image_data.is_jpeg_thumbnail() {
            let data_size = length as usize - IMAGE_HEADER_SIZE;
            image_data.data = self.read_bytes(data_size)?;
        }

        Ok(image_data)
    }

    fn read_property_list(&mut self, offset: u64) -> Result<Vec<X3fProperty>, X3fError> {
        // Go to the property list section.
        self.seek_to(offset)?;

        // Verify the section identifiers.
        let mut buf = [0; 4];
        self.inner.read_exact(&mut buf)?;
        if buf.cmp(&b"SECp") != Ordering::Equal {
            return Err(X3fError::Format("SECp not found".to_owned()));
        }

        // Verify the section version.
        let version = self.read_u32()?;
        let secp_version_str = format!("{:08x}", version);
        dbg!(secp_version_str);
        if version != 0x20000 {
            return Err(X3fError::Format("Unsupported SECp version".to_owned()));
        }

        // Read the property list information.
        let num_entries = self.read_u32()?;
        let character_encoding = self.read_u32()?;
        self.seek_by(4)?; // skip reserved
        let total_length = self.read_u32()?;
        dbg!(num_entries, character_encoding, total_length);
        if character_encoding != 0 {
            return Err(X3fError::Format(
                "Unsupported SECp character encoding".to_owned(),
            ));
        }

        // Read properties.
        let entries = self.read_property_entries(num_entries)?;
        let props = self.read_properties(num_entries as usize, total_length as usize)?;
        dbg!(&entries, &props);
        debug_assert_eq!(entries.len(), props.len());
        debug_assert_eq!(entries[0].value_offset - 1, props[0].name.len() as u32);

        Ok(props)
    }

    fn read_property_entries(
        &mut self,
        num_entries: u32,
    ) -> Result<Vec<X3fPropertyEntry>, io::Error> {
        let mut entries = Vec::new();
        for _ in 0..num_entries {
            let name_offset = self.read_u32()?;
            let value_offset = self.read_u32()?;
            let entry = X3fPropertyEntry {
                name_offset,
                value_offset,
            };
            entries.push(entry);
        }
        Ok(entries)
    }

    fn read_properties(
        &mut self,
        num_entries: usize,
        num_characters: usize,
    ) -> Result<Vec<X3fProperty>, io::Error> {
        // Read whole properties as bytes and convert it to string.
        let src_vec = self.read_bytes(num_characters * 2)?;
        let src = src_vec.as_slice();
        let mut dst_vec = Vec::<u16>::with_capacity(num_characters);
        unsafe {
            dst_vec.set_len(num_characters);
        }
        let mut dst = dst_vec.as_mut_slice();
        LittleEndian::read_u16_into(&src, &mut dst);
        let all_props = String::from_utf16(&dst).unwrap();

        // Make a property list.
        let mut props = Vec::new();
        let flatten_props: Vec<&str> = all_props.split('\0').collect();
        for pos in 0..num_entries {
            let name = flatten_props[pos * 2].to_owned();
            let value = flatten_props[pos * 2 + 1].to_owned();
            props.push(X3fProperty { name, value });
        }
        Ok(props)
    }

    fn read_bytes(&mut self, length: usize) -> Result<Vec<u8>, io::Error> {
        let mut buf = Vec::with_capacity(length);
        unsafe {
            buf.set_len(length);
        }
        self.inner.read_exact(&mut buf.as_mut_slice())?;
        Ok(buf)
    }

    //#[inline]
    //fn read_u16(&mut self) -> Result<u16, io::Error> {
    //    self.inner.read_u16::<LittleEndian>()
    //}

    #[inline]
    fn read_u32(&mut self) -> Result<u32, io::Error> {
        self.inner.read_u32::<LittleEndian>()
    }

    #[inline]
    fn seek_by(&mut self, pos: i64) -> Result<u64, io::Error> {
        self.inner.seek(SeekFrom::Current(pos))
    }

    #[inline]
    fn seek_to(&mut self, pos: u64) -> Result<u64, io::Error> {
        self.inner.seek(SeekFrom::Start(pos))
    }
}

#[derive(Debug)]
enum X3fError {
    Io(io::Error),
    Format(String),
}

impl std::fmt::Display for X3fError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            X3fError::Io(ref err) => err.fmt(f),
            X3fError::Format(ref s) => write!(f, "{}", s),
        }
    }
}

impl std::error::Error for X3fError {
    fn description(&self) -> &str {
        match *self {
            X3fError::Io(ref err) => err.description(),
            X3fError::Format(ref s) => s,
        }
    }

    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            X3fError::Io(ref err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for X3fError {
    fn from(err: io::Error) -> X3fError {
        X3fError::Io(err)
    }
}

impl From<String> for X3fError {
    fn from(err: String) -> X3fError {
        X3fError::Format(err)
    }
}
