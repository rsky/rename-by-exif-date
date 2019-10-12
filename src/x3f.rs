extern crate byteorder;
extern crate chrono;
extern crate derivative;
extern crate exif;

use super::exif::read_date_time_original_as_utc;
use byteorder::{LittleEndian, ReadBytesExt};
use chrono::DateTime;
use chrono_tz::Tz;
use derivative::Derivative;
use exif::Reader;
use std::cmp::Ordering;
//use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, BufReader, SeekFrom};

pub fn read_x3f_time(filename: &str, from_tz: Option<Tz>) -> Result<Option<DateTime<Tz>>, String> {
    let file = match File::open(filename) {
        Err(e) => return Err(e.to_string()),
        Ok(f) => f,
    };

    let reader = X3fPropReader::new(file, from_tz).map_err(|e| e.to_string())?;
    return Ok(reader.exif_datetime);
}

struct X3fPropReader<R: Read + Seek> {
    inner: R,
    from_tz: Option<Tz>,
    //props: HashMap<String, Vec<u8>>,
    exif_datetime: Option<DateTime<Tz>>,
}

#[derive(Debug)]
struct X3fDirectoryEntry {
    name: String,
    offset: u32,
    length: u32,
}

#[derive(Derivative)]
#[derivative(Debug)]
struct X3fImageData {
    image_type: u32,
    data_format: u32,
    columns: u32,
    rows: u32,
    row_stride: u32,
    #[derivative(Debug = "ignore")]
    data: Vec<u8>,
}

impl X3fImageData {
    pub fn is_jpeg_thumbnail(&self) -> bool {
        self.image_type == 2 /* thumbnail */ && self.data_format == 18 /* JPEG */
    }
}

impl<R: Read + Seek> X3fPropReader<R> {
    pub fn new(inner: R, from_tz: Option<Tz>) -> Result<Self, X3fError> {
        let mut reader = X3fPropReader {
            inner,
            from_tz,
            //props: HashMap::new(),
            exif_datetime: None,
        };
        reader.read_props()?;

        return Ok(reader);
    }

    fn read_props(&mut self) -> Result<(), X3fError> {
        self.check_identifier()?;
        let dir_offset = self.read_directory_pointer()?;
        dbg!(dir_offset);
        self.seek_to(dir_offset)?;

        let num_directory_entries = self.check_directory()?;
        dbg!(num_directory_entries);

        let dir_entries = self.read_directory_entries(num_directory_entries)?;
        dbg!(&dir_entries);

        for entry in dir_entries.iter() {
            match entry.name.as_str() {
                "PROP" => { /* TODO */ }
                "IMA2" => {
                    let image_data =
                        self.read_image_data(entry.offset as u64, entry.length as u64)?;
                    dbg!(&image_data);
                    if image_data.is_jpeg_thumbnail() {
                        self.exif_datetime = self.read_datetime_from_thumbnail(&image_data);
                    }
                }
                _ => {}
            }
        }

        return Ok(());
    }

    fn check_identifier(&mut self) -> Result<(), X3fError> {
        let mut buf = [0; 4];
        self.inner.read_exact(&mut buf)?;
        if buf.cmp(&b"FOVb") != Ordering::Equal {
            return Err(X3fError::Format("Not a X3F (FOVb) file".to_owned()));
        }

        let version = self.read_u32()?;
        let version_str = format!("{:08x}", version);
        dbg!(version_str);

        return Ok(());
    }

    fn check_directory(&mut self) -> Result<u32, X3fError> {
        let mut buf = [0; 4];
        self.inner.read_exact(&mut buf)?;
        if buf.cmp(&b"SECd") != Ordering::Equal {
            return Err(X3fError::Format("SECd not found".to_owned()));
        }

        let version = self.read_u32()?;
        let version_str = format!("{:08x}", version);
        dbg!(version_str);
        if version != 0x20000 {
            return Err(X3fError::Format("Unsupported SECd version".to_owned()));
        }

        let num_entries = self.read_u32()?;
        return Ok(num_entries);
    }

    fn read_directory_entries(
        &mut self,
        num_directory_entries: u32,
    ) -> Result<Vec<X3fDirectoryEntry>, X3fError> {
        let mut entries = Vec::new();
        for _ in 0..num_directory_entries {
            let offset = self.read_u32()?;
            let length = self.read_u32()?;
            let mut buf = [0; 4];
            self.inner.read_exact(&mut buf)?;
            let name = String::from_utf8_lossy(&buf);
            let entry = X3fDirectoryEntry {
                name: String::from(name),
                offset,
                length,
            };
            entries.push(entry);
        }
        return Ok(entries);
    }

    fn read_directory_pointer(&mut self) -> Result<u64, io::Error> {
        self.inner.seek(SeekFrom::End(-4))?;
        let offset = self.read_u32()?;
        return Ok(offset as u64);
    }

    fn read_datetime_from_thumbnail(&mut self, image_data: &X3fImageData) -> Option<DateTime<Tz>> {
        match Reader::new(&mut BufReader::new(image_data.data.as_slice())) {
            Ok(reader) => read_date_time_original_as_utc(&reader, self.from_tz),
            Err(_) => None,
        }
    }

    fn read_image_data(&mut self, offset: u64, length: u64) -> Result<X3fImageData, X3fError> {
        self.seek_to(offset)?;
        let mut buf = [0; 4];
        self.inner.read_exact(&mut buf)?;
        if buf.cmp(&b"SECi") != Ordering::Equal {
            return Err(X3fError::Format("SECi not found".to_owned()));
        }
        let version = self.read_u32()?;
        let version_str = format!("{:08x}", version);
        dbg!(version_str);
        if version != 0x20000 {
            return Err(X3fError::Format("Unsupported SECi version".to_owned()));
        }

        let image_type = self.read_u32()?;
        let data_format = self.read_u32()?;
        let columns = self.read_u32()?;
        let rows = self.read_u32()?;
        let row_stride = self.read_u32()?;

        let mut image_data = X3fImageData {
            image_type,
            data_format,
            columns,
            rows,
            row_stride,
            data: Vec::new(),
        };

        if image_data.is_jpeg_thumbnail() {
            image_data.data = self.read_bytes((length - 28) as usize)?;
        }

        return Ok(image_data);
    }

    fn read_bytes(&mut self, length: usize) -> Result<Vec<u8>, io::Error> {
        const BUF_SIZE_L: usize = 1 << 16;
        const BUF_SIZE_S: usize = 1 << 8;

        let mut buf = Vec::with_capacity(length);
        let mut b1 = [0; BUF_SIZE_L];
        let mut b2 = [0; BUF_SIZE_S];
        let mut b3 = [0; 1];
        for _ in 0..(length / BUF_SIZE_L) {
            self.inner.read_exact(&mut b1)?;
            buf.append(&mut b1.to_vec());
        }
        for _ in 0..((length % BUF_SIZE_L) / BUF_SIZE_S) {
            self.inner.read_exact(&mut b2)?;
            buf.append(&mut b2.to_vec());
        }
        for _ in 0..(length % BUF_SIZE_S) {
            self.inner.read_exact(&mut b3)?;
            buf.push(b3[0]);
        }
        debug_assert_eq!(buf.len(), length);
        return Ok(buf);
    }

    //#[inline]
    //fn read_u16(&mut self) -> Result<u16, io::Error> {
    //    self.inner.read_u16::<LittleEndian>()
    //}

    #[inline]
    fn read_u32(&mut self) -> Result<u32, io::Error> {
        self.inner.read_u32::<LittleEndian>()
    }

    //#[inline]
    //fn seek_by(&mut self, pos: i64) -> Result<u64, io::Error> {
    //    self.inner.seek(SeekFrom::Current(pos))
    //}

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
