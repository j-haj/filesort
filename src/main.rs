extern crate chrono;
extern crate chrono_tz;
extern crate clap;
extern crate rayon;

use chrono::{Date, Datelike, DateTime, NaiveDateTime};
use chrono::offset::Local;
use clap::{Arg, App};
use rayon::prelude::*;
use std::collections::HashSet;
use std::io;
use std::fs::{self, DirEntry};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, SystemTimeError, UNIX_EPOCH};

/**
 * Moves files from base directory `dir` to the path given by `move_to`.
 */
fn move_files(dir: &Path, should_move: &Fn(&Path)->bool,
              move_to: &Fn(&DirEntry)->Result<PathBuf, ()>, should_recurse: bool)
              -> io::Result<()> {
    if dir.is_dir() {
        // TODO: The issue is we are adding directories, then seeing them with read_dir and adding
        // to them recursively!
        fs::read_dir(dir).expect("Failed to read directory contents").for_each(|e| {
            let entry = e.unwrap();
            let path = entry.path();
            if path.is_dir() && should_recurse {
                move_files(&path, should_move, move_to, should_recurse)
                    .unwrap();
            } else {
                if should_move(&entry.path()) {
                    if let Ok(new_path) = move_to(&entry) {
                        fs::create_dir_all(&new_path.parent()
                                                    .expect(&format!("Failed with {:?}",
                                                                     new_path.parent())));
                        fs::rename(&path, &new_path).expect(&format!("Failed to rename to {:?}",
                                                                     new_path));
                    }
                }
            }
        });
    }
    Ok(())
}

fn is_image_file(path: &Path) -> bool {
    let image_extensions : HashSet<&str> = vec!["jpg", "jpeg", "tiff", "JPG", "JPEG", "TIFF", "mov", "MOV"]
        .into_iter()
        .collect();
    if path.is_file() {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            return image_extensions.contains(&ext);
        }
    }
    false
}

fn create_date_path(from: &DirEntry) -> Result<PathBuf, ()> {
    if let Ok(created) = from.metadata().and_then(|r| r.modified()) {
        if let Ok(date) = systemtime_to_date(created) {
            let ymd_path = format!("{}/{}/{}",
                                   date.year(),
                                   date.month(),
                                   date.day());
            let mut buf = from
                .path()
                .parent()
                .unwrap()
                .join(ymd_path);
            buf.push(from
                     .path()
                     .file_name()
                     .unwrap()
                     .to_str()
                     .unwrap());
            return Ok(buf)
        }
    } else {
        println!("Failed to fetch metadata for {:?}", from);
    }
    Err(())
}

fn systemtime_to_date(t: SystemTime) -> Result<Date<Local>, SystemTimeError> {
    match t.duration_since(UNIX_EPOCH) {
        Ok(dur) => {
            let dt = DateTime::<Local>::from_utc(NaiveDateTime::from_timestamp(
            dur.as_secs() as i64, dur.subsec_nanos()), *Local::now().offset());
            Ok(dt.date())
        }
        Err(e) => Err(e)
    }
}

fn main() {
    let matches = App::new("File Organizer")
        .version("1.0")
        .author("Jeff Hajewski")
        .about("Sorts files based on file metadata.")
        .arg(Arg::with_name("directory")
             .short("d")
             .long("dir")
             .value_name("DIRECTORY")
             .help("Directory containing files to sort")
             .required(true)
             .takes_value(true))
        .arg(Arg::with_name("recursive")
             .short("r")
             .long("recursive")
             .help("Dictates whether directories are recursively traversed")
             .default_value("true"))
        .get_matches();
    let directory = Path::new(matches.value_of("directory").unwrap());
    let should_recurse = matches.value_of("recursive").unwrap() == "true";
    println!("Running in directory: {:?} and will recurse: {}", directory,
             should_recurse);

    let _ = move_files(directory, &is_image_file, &create_date_path, should_recurse);
    println!("Done.");
}
