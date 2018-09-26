#!/usr/bin/env run-cargo-script
//cargo-deps: gnuplot="0.0.26"

//use std::io::prelude::*;
use std::process::Command;
//use std::{thread, time};

extern crate gnuplot;

fn main() {
	println!("script launched, args: {:?}", std::env::args().skip(1).collect::<String>());

	let idle_time: u64 = {
		let idle_time = Command::new("xprintidle").output().unwrap();
		assert!(idle_time.status.success());
		let idle_time = String::from_utf8(idle_time.stdout).unwrap();
		idle_time.trim().parse::<u64>().unwrap()
	};
	println!("idle_time: {}", idle_time);

	let (desktop_number, window_name) = {
		let command = Command::new("xdotool")
			.arg("getactivewindow")
			.arg("get_desktop")
			.arg("getwindowname")
			.output().unwrap();
		assert!(command.status.success());
		let stdout: String = String::from_utf8(command.stdout).unwrap();
		let stdout: &str = stdout.as_ref();
		let split: Vec<&str> = stdout.split('\n').collect();
		(split[0].parse::<u32>().unwrap(), split[1].to_string())
	};
	eprintln!("We're on desktop {:?} and our window is {:?}", desktop_number, window_name);

	use gnuplot::*;
	let x = [-10, -9, -8, -7, -6, -5, -4, -3, -2, -1, 0];
	let skip = [1, 0, 1, 1, 1, 1, 0, 1, 0, 0, 0];
	let work = [0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 1];
	let mut fg = Figure::new();
	fg.set_terminal("svg", "/home/vasya/.cache/timeplot/timeplot.svg");
	fg.axes2d()
//		.set_x_axis(false, &[])
//		.set_y_axis(false, &[Color("blue"), LineStyle(Dot)])
		.set_x_ticks(None, &[], &[TextColor("blue")]) // Some((Auto, 0))
		.set_y_ticks(None, &[], &[])
		.set_border(false, &[], &[])
//		.set_grid_options(false, &[LineStyle(Dot), Color("blue"), LineWidth(0.0)])
//		.set_x_grid(false)
//		.set_y_grid(false)
//		.set_pos_grid(1, 1, 0)
		.lines(&x, &skip, &[Caption(""), Color("orange"), PointSize(1.0), PointSymbol('*')])
		.lines(&x, &work, &[Caption(""), Color("black"), PointSize(1.0), PointSymbol('*')])
		.set_y_range(Fix(-0.1), Fix(10.1));
	fg.echo_to_file("/home/vasya/.cache/timeplot/timeplot.gp");
	fg.show();

//	let name_contains_closure = |pattern| regex::Regex::new(pattern).unwrap().is_match(test);

//	fn nameContains(pattern: &str) -> bool {
//		regex::Regex::new(pattern).unwrap().is_match(window_and_desktop);
//		return true
//	}

}
