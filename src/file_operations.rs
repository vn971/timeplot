extern crate chrono;
extern crate config;
extern crate directories;
extern crate env_logger;
extern crate fs2;
extern crate gnuplot;
extern crate open;
#[cfg(target_os = "windows")]
extern crate user32;
#[cfg(target_os = "windows")]
extern crate winapi;

use std::fs::OpenOptions;
use std::io::prelude::*;
use std::ops::Not;
use std::path::Path;
use std::path::PathBuf;

pub fn ensure_file(filename: &PathBuf, content: &str) {
	if Path::new(&filename).exists().not() {
		let mut file = OpenOptions::new()
			.create(true)
			.write(true)
			.open(filename)
			.unwrap_or_else(|err| panic!("Failed to open or create file {:?}, {}", filename, err));
		file.write_all(content.as_bytes())
			.unwrap_or_else(|err| panic!("failed to write to file {:?}, {}", filename, err));
	}
}
