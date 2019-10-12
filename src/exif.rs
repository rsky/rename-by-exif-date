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
    if let Some(dto) = reader.get_field(Tag::DateTimeOriginal, false) {
        let dt_str = dto.value.display_as(dto.tag).to_string();
        dbg!(&dt_str);
        let naive_dt = NaiveDateTime::parse_from_str(&dt_str, "%Y-%m-%d %H:%M:%S").unwrap();
        let utc: DateTime<Tz>;
        if let Some(tz) = from_tz {
            let dt = tz.from_local_datetime(&naive_dt).unwrap();
            utc = dt.with_timezone(&UTC);
        } else if let Some(oto) = reader.get_field(Tag::OffsetTimeOriginal, false) {
            let dt_tz_str = format!("{}{}", dt_str, oto.value.display_as(oto.tag));
            dbg!(&dt_tz_str);
            let dt = DateTime::parse_from_str(&dt_tz_str, "%Y-%m-%d %H:%M:%S%:z").unwrap();
            utc = dt.with_timezone(&UTC);
        } else {
            let dt = Local.from_local_datetime(&naive_dt).unwrap();
            utc = dt.with_timezone(&UTC);
        }
        return Ok(Some(utc));
    }
    return Ok(None);
}
