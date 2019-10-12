extern crate chrono;
extern crate chrono_tz;
extern crate exif;

use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use chrono_tz::{Tz, UTC};
use exif::{Reader, Tag};
use std::fs::File;
use std::io::BufReader;

pub fn read_exif_date_time_original(
    filename: &str,
    from_tz: Option<Tz>,
) -> Result<(Option<DateTime<Tz>>), String> {
    let file = match File::open(filename) {
        Err(e) => return Err(e.to_string()),
        Ok(f) => f,
    };

    let reader = match Reader::new(&mut BufReader::new(&file)) {
        Err(e) => return Err(e.to_string()),
        Ok(r) => r,
    };

    return Ok(read_date_time_original_as_utc(&reader, from_tz));
}

fn read_date_time_original_as_utc(reader: &Reader, from_tz: Option<Tz>) -> Option<DateTime<Tz>> {
    let date_time_original = reader.get_field(Tag::DateTimeOriginal, false);
    if let Some(dto) = date_time_original {
        let offset_time_original = reader.get_field(Tag::OffsetTimeOriginal, false);
        return Some(match (from_tz, offset_time_original) {
            (Some(tz), _) => utc_date_time_original_with_timezone(&dto, &tz),
            (None, Some(oto)) => utc_date_time_original_with_offset(&dto, &oto),
            (None, None) => utc_date_time_original(&dto),
        });
    }
    return None;
}

fn field_as_string(field: &exif::Field) -> String {
    return field.value.display_as(field.tag).to_string();
}

fn date_time_original_as_naive(dto: &exif::Field) -> NaiveDateTime {
    let dt_str = field_as_string(&dto);
    dbg!(&dt_str);
    return NaiveDateTime::parse_from_str(&dt_str, "%Y-%m-%d %H:%M:%S").unwrap();
}

fn utc_date_time_original(dto: &exif::Field) -> DateTime<Tz> {
    return Local
        .from_local_datetime(&date_time_original_as_naive(dto))
        .unwrap()
        .with_timezone(&UTC);
}

fn utc_date_time_original_with_timezone(dto: &exif::Field, tz: &Tz) -> DateTime<Tz> {
    return tz
        .from_local_datetime(&date_time_original_as_naive(dto))
        .unwrap()
        .with_timezone(&UTC);
}

fn utc_date_time_original_with_offset(dto: &exif::Field, oto: &exif::Field) -> DateTime<Tz> {
    let dt_str = format!("{}{}", field_as_string(dto), field_as_string(oto));
    dbg!(&dt_str);
    return DateTime::parse_from_str(&dt_str, "%Y-%m-%d %H:%M:%S%:z")
        .unwrap()
        .with_timezone(&UTC);
}
