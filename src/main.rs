#!/usr/bin/env run-cargo-script
//cargo-deps: gnuplot="0.0.26"

use std::io::prelude::*;
use std::process::Command;
use std::time::SystemTime;
use std::fs::OpenOptions;

extern crate gnuplot;


fn do_plot(time: u64) {
	use gnuplot::*;
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
		.label(&format!("{:?}", time), Graph(0.0), Graph(-0.02), &[])
		.set_y_range(Fix(-0.1), Fix(10.1));
	 fg.echo_to_file(home.join(".cache/timeplot/timeplot.gnuplot").to_str().unwrap());
	fg.show();
}

fn get_category(_desktop_number: u32, _window_name: &str) -> String {
	"skip".to_string()
}


fn do_save_current(time: u64) {
	let idle_time: u64 = {
		let idle_time = Command::new("xprintidle").output().unwrap();
		assert!(idle_time.status.success());
		let idle_time = String::from_utf8(idle_time.stdout).unwrap();
		idle_time.trim().parse::<u64>().unwrap()
	};
	eprintln!("idle_time: {}", idle_time);
	if idle_time > 1000 * 60 * 3 { // 3min
		return;
	}
	{
		assert!(1 == 1);
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

	let home = std::env::home_dir().unwrap();
	let home = home.as_path();
	let mut file = OpenOptions::new()
		.append(true).create(true)
		.open(home.join(".local/share/timeplot/log.log")).unwrap();
	writeln!(file, "{} {} {} {}",
		time,
		get_category(desktop_number, &window_name),
		desktop_number,
		window_name
	).unwrap();
}


fn main() {
	eprintln!("script launched, args: {:?}", std::env::args().skip(1).collect::<String>());

	//const readme: &'static str = include_str!("../README.txt");

	//if std::env::args().nth(1) == Some("--help".to_string()) {
	//	eprintln!("");
	//	return;
	//}
	// TODO: parse args

	let home = std::env::home_dir().unwrap();
	let home = home.as_path();
	std::fs::create_dir_all(home.join(".config/timeplot")).unwrap();
	std::fs::create_dir_all(home.join(".cache/timeplot")).unwrap();
	std::fs::create_dir_all(home.join(".local/share/timeplot")).unwrap();
	// TODO: take file lock

	let time = std::time::SystemTime::now();
	let time = time.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();

	do_save_current(time);
	do_plot(time);

//	let name_contains_closure = |pattern| regex::Regex::new(pattern).unwrap().is_match(test);

//	fn nameContains(pattern: &str) -> bool {
//		regex::Regex::new(pattern).unwrap().is_match(window_and_desktop);
//		return true
//	}
}
