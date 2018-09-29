#!/usr/bin/env run-cargo-script
//cargo-deps: gnuplot="0.0.26"

use std::io::prelude::*;
use std::process::Command;
use std::time::SystemTime;
use std::fs::OpenOptions;
use std::path::Path;
use std::fs::File;
use std::io::BufReader;

extern crate gnuplot;


fn do_plot() {
	use gnuplot::*;
	let height_scale = 10.0; // TODO: parse from config
	let home = std::env::home_dir().unwrap();
	let home = home.as_path();
	let svg_file = home.join(".cache/timeplot/timeplot.svg");
	std::fs::remove_file(&svg_file).is_ok();
	let x = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, ];
	let skip = [1, 0, 1, 1, 1, 1, 0, 1, 0, 0, 0, 1, 0, 1, 1, 1, 1, 0, 1, 0, 0, 1, 0, 1, 1, 1, 1, 0, 1, ];
	let work = [0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, ];
	let mut fg = Figure::new();
	fg.set_terminal("svg", svg_file.to_str().unwrap());
	fg.axes2d()
		.set_x_ticks(None, &[], &[])
		.set_y_ticks(None, &[], &[])
		.set_x_log(Some(2.0))
		.set_border(false, &[], &[])
		.lines(&x, &skip, &[Caption(""), Color("orange"), PointSize(1.0), PointSymbol('*')])
		.lines(&x, &work, &[Caption(""), Color("black"), PointSize(1.0), PointSymbol('*')])
		.set_y_range(Fix(-0.1), Fix(height_scale));
	fg.echo_to_file(home.join(".cache/timeplot/gnuplot").to_str().unwrap());
	fg.show();
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
			let desktop_pattern = *split.get(1).expect(&parse_error);
			let window_pattern: &str = *split.get(2).unwrap_or(&"");
			let window_pattern = window_pattern.to_lowercase();
			let window_pattern = window_pattern.as_str();
			if (desktop_pattern == "-" || desktop_pattern == desktop_number.to_string())
				&& window_name.contains(window_pattern) {
				return category.to_string();
			}
		}
		panic!("Could not find any category for desktop {}, window {}", desktop_number, window_name);
	}
}


fn do_save_current() {
	{
		// TODO: ignore xprintidle flag
		let idle_time = Command::new("xprintidle").output().unwrap();
		assert!(idle_time.status.success());
		let idle_time = String::from_utf8(idle_time.stdout).unwrap();
		let idle_time = idle_time.trim().parse::<u64>().unwrap();
		eprintln!("idle_time: {}", idle_time);
		if idle_time > 1000 * 60 * 3 { // 3min
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
		let window_name = split[1].replace("\n", "").as_str().chars().take(200).collect::<String>();
		(split[0].parse::<u32>().unwrap(), window_name)
	};
	eprintln!("We're on desktop {} and our window is {}", desktop_number, window_name);
	std::env::set_var("DESKTOP_NUMBER", desktop_number.to_string());
	std::env::set_var("WINDOW_NAME", &window_name);

	let home = std::env::home_dir().unwrap();
	let home = home.as_path();
	let mut file = OpenOptions::new()
		.append(true).create(true)
		.open(home.join(".local/share/timeplot/log.log")).unwrap();
	writeln!(file, "{} {} {} {}",
		std::time::SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(),
		get_category(desktop_number, &window_name),
		desktop_number,
		window_name
	).unwrap();
}


fn main() {
	eprintln!("script launched, args: {:?}", std::env::args().skip(1).collect::<String>());

	//const readme: &'static str = include_str!("../README.txt");

	let home = std::env::home_dir().unwrap();
	let home = home.as_path();
	std::fs::create_dir_all(home.join(".config/timeplot")).unwrap();
	std::fs::create_dir_all(home.join(".cache/timeplot")).unwrap();
	std::fs::create_dir_all(home.join(".local/share/timeplot")).unwrap();
	// TODO: take file lock

	// TODO: add XDG autostart. After explicit approval only?

	loop {
		do_save_current();
		do_plot();
		let duration = std::time::Duration::from_millis(1000*60*5); // TODO: configuration
		std::thread::sleep(duration);
	}

//	let name_contains_closure = |pattern| regex::Regex::new(pattern).unwrap().is_match(test);

//	fn nameContains(pattern: &str) -> bool {
//		regex::Regex::new(pattern).unwrap().is_match(window_and_desktop);
//		return true
//	}
}
