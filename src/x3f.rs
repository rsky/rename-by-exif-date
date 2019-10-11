extern crate chrono;
use chrono::{DateTime, Local};
use chrono_tz::{Tz, UTC};
use std::fs::File;
use std::io::BufReader;

pub fn read_x3f_time(filename: &str, from_tz: Option<Tz>) -> Result<Option<DateTime<Tz>>, String> {
    let file = match File::open(filename) {
        Err(e) => return Err(e.to_string()),
        Ok(f) => f,
    };

    let reader = BufReader::new(file);

    return Ok(None);
}
