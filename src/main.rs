#!/usr/bin/env run-cargo-script
//cargo-deps: gnuplot="0.0.26"

use std::cmp::min;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::BufReader;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::path::Path;
use std::process::Command;
use std::time::SystemTime;
use chrono::prelude::*;

extern crate fs2;
extern crate gnuplot;
extern crate chrono;

use fs2::FileExt;
use std::time::Duration;
use chrono::prelude::Utc;

struct LogEntry {
	time: u64,
	category: String,
	// desktop: u64,  // no need to parse
	// window_name: String,  // no need to parse
}

const FILE_SEEK: u64 = 100_000;
const PLOT_DAYS: u64 = 14;
const PLOT_HEIGHT_SCALE: f64 = 10.0;
const WINDOW_MAX_LENGTH: usize = 120;
const DATE_FORMAT: &str = "%Y-%m-%d_%H:%M";

fn parse_log_line(line: &str) -> LogEntry {
	let split: Vec<&str> = line.splitn(4, ' ').collect();
	let parse_error = format!("Failed to parse log entry {}", line);
	let time = Utc.datetime_from_str(split.get(0).expect(&parse_error), DATE_FORMAT).expect(&parse_error);
	LogEntry {
		time: time.timestamp_millis() as u64 / 1000,
		category: (*split.get(1).expect(&parse_error)).to_string(),
		// desktop: split.get(2).expect(&parse_error).parse::<u64>().unwrap(),
		// window_name: (*split.get(3).expect(&parse_error)).to_string(),
	}
}

struct CategoryData {
	category_name: String,
	color: String,
	value: Option<f32>,
	points: Vec<f32>,
}

fn do_plot() {
	use gnuplot::*;
	let home = std::env::home_dir().unwrap();
	let home = home.as_path();
	let svg_file = home.join(".cache/timeplot/timeplot.svg");

	let time_now = std::time::SystemTime::now()
		.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
	let min_time = time_now - PLOT_DAYS * 60 * 60 * 24;
	let log_file = File::open(home.join(".local/share/timeplot/log.log")).unwrap();
	let mut log_file = BufReader::new(log_file);

	// seek forward until we reach entries
	let mut pos = 0;
	loop {
		pos += FILE_SEEK;
		log_file.seek(SeekFrom::Start(pos)).unwrap();
		let mut line = String::new();
		log_file.read_line(&mut String::new()).unwrap();
		log_file.read_line(&mut line).unwrap();
		if line.is_empty() || parse_log_line(&line).time > min_time {
			log_file.seek(SeekFrom::Start(pos - FILE_SEEK)).unwrap();
			break;
		}
	}

	let mut categories: Vec<CategoryData> = Vec::new();
	categories.push(CategoryData {
		category_name: "work".to_string(),
		color: "black".to_string(),
		value: None,
		points: Vec::new(),
	});
	categories.push(CategoryData {
		category_name: "personal".to_string(),
		color: "orange".to_string(),
		value: None,
		points: Vec::new(),
	});
	categories.push(CategoryData {
		category_name: "fun".to_string(),
		color: "red".to_string(),
		value: None,
		points: Vec::new(),
	});

	let mut lines: Vec<_> = log_file.lines().map(|l| parse_log_line(&l.unwrap())).collect();
	lines.reverse();

	let mut last_time = time_now;
	let mut x_coord = Vec::new();
	for line in lines {
		if line.time < min_time { continue; }
		let time_diff = last_time - min(line.time, last_time); // TODO
		let weight_old = 1.0 / (time_diff as f32 / 300.0).exp();
		let weight_new = 1.0 - weight_old;
		for category in categories.iter_mut() {
			let new_value = if category.value.is_some() || line.category == category.category_name {
				let latest = if line.category == category.category_name { 1.0 } else { 0.0 };
				let old_value = category.value.unwrap_or(latest);
				Some(latest * weight_new + old_value * weight_old)
			} else {
				None
			};
			category.value = new_value;
			category.points.push(new_value.unwrap_or(0.0));
		}
		x_coord.push(line.time);
		last_time = line.time;
	}

	let mut figure = Figure::new();
	figure.set_terminal("svg", svg_file.to_str().unwrap());
	for category in categories {
		figure.axes2d()
			.set_x_ticks(None, &[], &[])
			.set_y_ticks(None, &[], &[])
			.set_border(false, &[], &[])
			.lines(&x_coord, &category.points, &[Caption(""), Color(&category.color), PointSize(1.0), PointSymbol('*')])
			.set_y_range(Fix(-0.1), Fix(PLOT_HEIGHT_SCALE));
	}
	figure.echo_to_file(home.join(".cache/timeplot/gnuplot").to_str().unwrap());
	figure.show();
}

fn get_category(desktop_number: u32, window_name: &str) -> String {
	let window_name = window_name.to_lowercase();
	let window_name = window_name.as_str();
	let home = std::env::home_dir().unwrap();
	let home = home.as_path();
	if Path::new(&home.join(".config/timeplot/category_decider")).exists() {
		let child = Command::new(home.join(".config/timeplot/category_decider")).output().unwrap();
		assert!(child.status.success());
		String::from_utf8(child.stdout).unwrap()
	} else {
		// TODO: create rules_simple.txt if it does not exist
		let rules_file = File::open(home.join(".config/timeplot/rules_simple.txt")).unwrap();
		let rules_file = BufReader::new(rules_file);

		for line in rules_file.lines() {
			let line = line.unwrap();
			if line.starts_with("#") || line.is_empty() {
				continue;
			}
			let split: Vec<&str> = line.splitn(3, ' ').collect();
			let parse_error = format!("Cannot parse ~/.config/timeplot/rules_simple.txt, line: {}", line);
			let category = *split.get(0).expect(&parse_error);
			let window_pattern = *split.get(1).unwrap_or(&"");
			let window_pattern = window_pattern.to_lowercase();
			if window_name.contains(&window_pattern) {
				return category.to_string();
			}
		}
		eprintln!("Could not find any category for desktop {}, window {}", desktop_number, window_name);
		"skip".to_string()
	}
}


fn do_save_current() {
	{
		// TODO: allowing ignore xprintidle
		let idle_time = Command::new("xprintidle").output().unwrap();
		assert!(idle_time.status.success());
		let idle_time = String::from_utf8(idle_time.stdout).unwrap();
		let idle_time = idle_time.trim().parse::<u64>().unwrap() / 1000;
		if idle_time > 60 * 3 { // 3min
			eprintln!("skipping log due to idle time: {}", idle_time);
			return;
		}
	}

	let (desktop_number, window_name) = {
		let command = Command::new("xdotool")
			.arg("getactivewindow")
			.arg("get_desktop")
			.arg("getwindowname")
			.output().unwrap();
		assert!(command.status.success());
		let stdout = String::from_utf8_lossy(&command.stdout);
		let split: Vec<&str> = stdout.split('\n').collect();
		let window_name = split[1].replace("\n", "").as_str().chars()
			.take(WINDOW_MAX_LENGTH).collect::<String>();
		(split[0].parse::<u32>().unwrap(), window_name)
	};
	// eprintln!("We're on desktop {} and our window is {}", desktop_number, window_name);
	std::env::set_var("DESKTOP_NUMBER", desktop_number.to_string());
	std::env::set_var("WINDOW_NAME", &window_name);

	let home = std::env::home_dir().unwrap();
	let home = home.as_path();
	let mut file = OpenOptions::new()
		.append(true).create(true)
		.open(home.join(".local/share/timeplot/log.log")).unwrap();
	let log_line = format!("{} {} {} {}",
		Utc::now().format(DATE_FORMAT),
		get_category(desktop_number, &window_name),
		desktop_number,
		window_name);
	eprintln!("logging: {}", log_line);
	file.write_all(log_line.as_bytes()).unwrap();
	file.write_all("\n".as_bytes()).unwrap();
}


fn main() {
	eprintln!("script launched, args: {:?}", std::env::args().skip(1).collect::<String>());

	//const readme: &'static str = include_str!("../README.txt");
	let home = std::env::home_dir().unwrap();
	let home = home.as_path();
	std::fs::create_dir_all(home.join(".config/timeplot")).unwrap();
	std::fs::create_dir_all(home.join(".cache/timeplot")).unwrap();
	std::fs::create_dir_all(home.join(".local/share/timeplot")).unwrap();
	File::open(home.join(".config/timeplot/")).unwrap()
		.try_lock_exclusive().expect("Another instance of timeplot is already running.");

	// TODO: add XDG autostart. After explicit approval only?  $XDG_CONFIG_HOME/autostart

	loop {
		do_save_current();
		do_plot();
		let duration = Duration::from_secs(60 * 5); // TODO: configuration
		std::thread::sleep(duration);
	}
}
