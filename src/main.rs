extern crate chrono;
extern crate chrono_tz;
extern crate clap;
use chrono::TimeZone;
use chrono_tz::Tz;
use clap::{App, Arg, ArgMatches};
use std::process;

fn main() {
    let matches = app().get_matches();
    let tz = get_tz(matches);
    match tz {
        Some((f, t)) => println!(
            "{}",
            f.ymd(2016, 10, 22)
                .and_hms(12, 0, 0)
                .with_timezone(&t)
                .to_string(),
        ),
        None => {}
    }
}

fn get_tz(matches: ArgMatches) -> Option<(Tz, Tz)> {
    if !matches.is_present("from-tz") {
        return None;
    }
    let from_tz: Result<Tz, ()> = match matches.value_of("from-tz").unwrap().parse() {
        Err(e) => Err(eprintln!("Failed to parse from-tz: {}", e)),
        Ok(tz) => Ok(tz),
    };
    let to_tz: Result<Tz, ()> = match matches.value_of("to-tz").unwrap().parse() {
        Err(e) => Err(eprintln!("Failed to parse to-tz: {}", e)),
        Ok(tz) => Ok(tz),
    };
    return match (from_tz, to_tz) {
        (Ok(f), Ok(t)) => Some((f, t)),
        _ => process::exit(1),
    };
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
                .help("Convert the time zone of the `DateTimeOriginal`")
                .display_order(3)
                .long("from-tz")
                .requires("to-tz")
                .takes_value(true)
                .empty_values(false),
        )
        .arg(
            Arg::with_name("to-tz")
                .help("EXIF tag from `from-tz` to `to-tz` if it is present")
                .display_order(4)
                .long("to-tz")
                .requires("from-tz")
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
