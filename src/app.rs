extern crate chrono_tz;
extern crate clap;
use chrono_tz::Tz;
use clap::{App, Arg, ArgMatches};
use std::collections::HashSet;
use std::path::Path;
use std::process;

pub fn app<'a, 'b>() -> App<'a, 'b> {
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
            Arg::with_name("dirname-format")
                .help("Specifies the format of the directory name")
                .display_order(0)
                .long("dirname-format")
                .default_value("%Y%m%d-%H%M%S"),
        )
        .arg(
            Arg::with_name("filename-format")
                .help("Specifies the format of the filename. (case insensitive and no dot)")
                .display_order(1)
                .long("filename-format")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("extensions")
                .help("Specifies the filename extension to handle.")
                .display_order(2)
                .long("ext")
                .short("e")
                .takes_value(true)
                .multiple(true),
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
                .display_order(4)
                .long("collision")
                .possible_values(&["overwrite", "skip", "serial", "abort"])
                .default_value("overwrite"),
        )
        .arg(
            Arg::with_name("from-tz")
                .help("FIXME")
                .display_order(5)
                .long("from-tz")
                .takes_value(true)
                .empty_values(false),
        )
        .arg(
            Arg::with_name("to-tz")
                .help("FIXME")
                .display_order(6)
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

pub fn get_timezones(matches: &ArgMatches) -> (Option<Tz>, Option<Tz>) {
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

pub fn get_filename_filter(matches: &ArgMatches) -> Box<dyn Fn(&Path) -> bool> {
    if !matches.is_present("extensions") {
        return Box::new(|_: &Path| true);
    }

    let extensions: HashSet<_> = matches
        .values_of("extensions")
        .unwrap()
        .map(|ext| ext.to_lowercase())
        .collect();

    return Box::new(move |path: &Path| match path.extension() {
        Some(ext) => extensions.contains(&ext.to_str().unwrap().to_lowercase()),
        None => false,
    });
}
