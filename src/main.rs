extern crate chrono;
mod app;
mod exif;
mod x3f;

use self::app::{app, get_extension_filter, get_timezones};
use self::exif::read_exif_date_time_original;
use self::x3f::read_x3f_time;
use chrono::Local;
use std::path::Path;
use std::process;

fn main() {
    let matches = app().get_matches();
    let (from_tz, to_tz) = get_timezones(&matches);
    let filer_fn = get_extension_filter(&matches);
    let sources = matches.values_of("sources").unwrap();
    for filename in sources {
        let path = Path::new(&filename);
        let ext = path.extension().unwrap_or_default().to_string_lossy();
        let lcext = ext.to_lowercase();
        if !filer_fn(&lcext) {
            continue;
        }
        let dt = if lcext == "x3f" {
            read_x3f_time(filename, from_tz)
        } else {
            read_exif_date_time_original(filename, from_tz)
        };
        match dt {
            Ok(dt) => match dt {
                Some(dt) => match to_tz {
                    Some(tz) => println!("{} -> {}", filename, dt.with_timezone(&tz)),
                    None => println!("{} -> {}", filename, dt.with_timezone(&Local)),
                },
                None => println!("{} -> none", filename),
            },
            Err(e) => {
                eprintln!("{}", e);
                process::exit(1);
            }
        }
    }
}
