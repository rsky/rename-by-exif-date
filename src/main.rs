extern crate chrono;
extern crate chrono_tz;
extern crate clap;
extern crate exif;
mod x3f;
use self::x3f::read_x3f_time;
use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use chrono_tz::{Tz, UTC};
use clap::{App, Arg, ArgMatches};
use exif::{Reader, Tag};
use std::fs::File;
use std::io::BufReader;
use std::process;

fn main() {
    let matches = app().get_matches();
    let (from_tz, to_tz) = get_tz(&matches);
    let sources = matches.values_of("sources").unwrap();
    for filename in sources {
        let dt = if filename.to_lowercase().ends_with(".x3f") {
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

fn get_tz(matches: &ArgMatches) -> (Option<Tz>, Option<Tz>) {
    let from_tz: Result<Option<Tz>, ()> = match matches.value_of("from-tz") {
        None => Ok(None),
        Some(t) => match t.parse() {
            Ok(tz) => Ok(Some(tz)),
            Err(e) => Err(eprintln!("Failed to parse from-tz: {}", e)),
        },
    };
    let to_tz: Result<Option<Tz>, ()> = match matches.value_of("to-tz") {
        None => Ok(None),
        Some(t) => match t.parse() {
            Ok(tz) => Ok(Some(tz)),
            Err(e) => Err(eprintln!("Failed to parse to-tz: {}", e)),
        },
    };
    return match (from_tz, to_tz) {
        (Ok(f), Ok(t)) => (f, t),
        _ => process::exit(1),
    };
}

fn read_exif_date_time_original(
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

fn app<'a, 'b>() -> App<'a, 'b> {
    return App::new("Rename by EXIF")
        .version("0.1.0")
        .arg(
            Arg::with_name("destination")
                .help("Rename destination directory")
                .value_name("DESTINATION")
                .required(true)
                .empty_values(false)
                .index(1),
        )
        .arg(
            Arg::with_name("sources")
                .help("Rename source directories/filenames")
                .value_name("SOURCES")
                .required(true)
                .empty_values(false)
                .multiple(true),
        )
        .arg(
            Arg::with_name("copy")
                .help("Copies given image files instead of renaming")
                .long("copy")
                .short("c"),
        )
        .arg(
            Arg::with_name("subdir-by-date")
                .help("Makes a sub directory according to date time")
                .long("subdir-by-date")
                .short("d"),
        )
        .arg(
            Arg::with_name("subdir-format")
                .help("Specifies the format of the sub directory name")
                .display_order(1)
                .long("subdir-format")
                .default_value("%Y%m%d-%H%M%S"),
        )
        .arg(
            Arg::with_name("recursive")
                .help("Recursive (FIXME)")
                .long("recursive")
                .short("r"),
        )
        .arg(
            Arg::with_name("collision")
                .help("How to handle filename collision (FIXME)")
                .display_order(2)
                .long("collision")
                .possible_values(&["overwrite", "skip", "serial", "abort"])
                .default_value("overwrite"),
        )
        .arg(
            Arg::with_name("from-tz")
                .help("FIXME")
                .display_order(3)
                .long("from-tz")
                .takes_value(true)
                .empty_values(false),
        )
        .arg(
            Arg::with_name("to-tz")
                .help("FIXME")
                .display_order(4)
                .long("to-tz")
                .takes_value(true)
                .empty_values(false),
        )
        .arg(
            Arg::with_name("verbose")
                .help("Verbose outut (FIXME)")
                .long("verbose")
                .short("v"),
        )
        .arg(
            Arg::with_name("dry-run")
                .help("Dry run (FIXME)")
                .long("dry-run")
                .short("n"),
        );
}
