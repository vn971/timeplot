#!/usr/bin/env run-cargo-script

//use std::io::prelude::*;
use std::process::Command;
//use std::process::Stdio;
//use std::{thread, time};

//extern crate regex;

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

//	let name_contains_closure = |pattern| regex::Regex::new(pattern).unwrap().is_match(test);

//	fn nameContains(pattern: &str) -> bool {
//		regex::Regex::new(pattern).unwrap().is_match(window_and_desktop);
//		return true
//	}

	//	let command = Command::new("xprintidle").output().unwrap();

	//	let window_name = Command::new("xdotool")
	//			.arg("getactivewindow").arg("getwindowname")
	//			.output().unwrap().stdout;
	//	let window_name = String::from_utf8_lossy(&window_name);
	//	let window_name = window_name.trim();
	//	println!("current window: {}", window_name);
}
