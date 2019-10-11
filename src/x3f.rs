extern crate chrono;
use chrono::{NaiveDateTime};
use std::fs::File;
use std::io::BufReader;

pub fn read_x3f_time( filename: &str,) -> Result<Option<NaiveDateTime>, String> {
    let file = match File::open(filename) {
        Err(e) => return Err(e.to_string()),
        Ok(f) => f,
    };

    let reader = BufReader::new(file);

    return Ok(None);
}
