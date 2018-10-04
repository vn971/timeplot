#[global_allocator]
static GLOBAL: std::alloc::System = std::alloc::System;

extern crate chrono;
extern crate config;
extern crate directories;
extern crate fs2;
extern crate gnuplot;

use chrono::prelude::*;
use config::Config;
use directories::ProjectDirs;
use directories::UserDirs;
use fs2::FileExt;
use std::cmp::min;
use std::collections::HashMap;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::BufReader;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

const WINDOW_MAX_LENGTH: usize = 120;
const FILE_SEEK: u64 = 100_000;
const DATE_FORMAT: &str = "%Y-%m-%d_%H:%M"; // cannot change without losing backwards compat
const LOG_FILE_NAME: &str = "log.log";

const RULES_FILE_NAME: &str = "rules_simple.txt";
const RULES_EXAMPLE: &'static str = include_str!("../example_rules_simple.txt");

const CONFIG_FILE_NAME: &str = "config.toml";
const CONFIG_EXAMPLE: &'static str = include_str!("../example_config.toml");
const CONFIG_PARSE_ERROR: &str = "Failed to parse config file. Consider removing/renaming it so it'll be recreated.";

struct LogEntry {
	/// EPOCH seconds
	time: u64,
	category: String,
	// desktop: u64,  // no need to parse
	// window_name: String,  // no need to parse
}

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
	time_impact: u64,
	value: Option<f32>,
	points: Vec<f32>,
}

fn do_plot(dirs: &ProjectDirs, conf: &Config) {
	use gnuplot::*;
	let conf_sleep_seconds = conf.get_float("main.sleep_minutes").expect(CONFIG_PARSE_ERROR);
	let conf_sleep_seconds = (conf_sleep_seconds * 60.0) as u64;

	let time_now = Utc::now().timestamp_millis() as u64 / 1000;
	let min_time = {
		let plot_days = conf.get_float("main.plot_days").expect(CONFIG_PARSE_ERROR);
		time_now - (plot_days * 60.0 * 60.0 * 24.0) as u64
	};
	let log_file = File::open(dirs.data_local_dir().join(LOG_FILE_NAME)).unwrap();
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

	let mut lines: Vec<_> = log_file.lines().map(|l| parse_log_line(&l.unwrap())).collect();
	lines.reverse();

	let mut categories: HashMap<&str, CategoryData> = HashMap::new();

	let mut last_time = time_now;
	let mut is_reset = true;
	let mut x_coord: Vec<f64> = Vec::new();
	for line in &lines {
		if line.time < min_time { continue; }
		if !conf.get_bool(&format!("category.{}.hide", &line.category)).unwrap_or(false)
			&& categories.contains_key(line.category.as_str()) == false {
			categories.insert(&line.category, CategoryData {
				category_name: line.category.to_string(),
				color: conf.get_str(&format!("category.{}.color", &line.category)).unwrap_or("black".to_string()).to_string(),
				time_impact: 0,
				value: None,
				points: Vec::new(),
			});
		}
		let time_diff = last_time - min(line.time, last_time);
		// TODO: add two artificial graph points if `is_reset`
		let weight_old = 1.0 / (time_diff as f32 / 300.0).exp2();
		let weight_new = 1.0 - weight_old;

		for category in categories.values_mut() {
			if line.category == category.category_name {
				category.time_impact += min(time_diff, conf_sleep_seconds);
			};
			let latest = if line.category == category.category_name { 1.0 } else { 0.0 };
			let old_value = category.value.unwrap_or(if is_reset { latest } else { 0.0 });
			let new_value = Some(latest * weight_new + old_value * weight_old);
			category.value = new_value;
			category.points.push(new_value.unwrap_or(0.0));
		}
		x_coord.push((line.time as f64 - time_now as f64) / 60.0 / 60.0 / 24.0); // show "days" ticks
		is_reset = time_diff as f64 > 5.0 * 60.0 * conf.get_float("main.sleep_minutes").expect(CONFIG_PARSE_ERROR);
		last_time = line.time;
	}

	let mut figure = Figure::new();
	// "svg size 1000 1000"
	let extension = conf.get_str("graph.extension").expect(CONFIG_PARSE_ERROR);
	let plot_file_name = format!("{}.{}", extension, extension);

	let size_override = conf.get_str("graph.size").expect(CONFIG_PARSE_ERROR);
	let size_override = size_override.trim();
	let terminal = if size_override.is_empty() {
		format!("{}", extension)
	} else {
		format!("{} size {}", extension, size_override)
	};
	figure.set_terminal(&terminal, dirs.cache_dir().join(plot_file_name).to_str().unwrap());
	let show_days = conf.get_bool("graph.show_day_labels").expect(CONFIG_PARSE_ERROR);
	let show_hours = conf.get_bool("graph.show_category_hours").expect(CONFIG_PARSE_ERROR);
	let show_names = conf.get_bool("graph.show_category_names").expect(CONFIG_PARSE_ERROR);
	{
		let axes = figure.axes2d()
			.set_y_ticks(None, &[], &[])
			.set_border(false, &[], &[])
			.set_y_range(Fix(-0.1), Fix(conf.get_float("graph.height_scale").expect(CONFIG_PARSE_ERROR)));
		if show_days {
			axes.set_x_ticks(Some((Auto, 0)), &[OnAxis(false), Inward(false), Mirror(false)], &[]);
		} else {
			axes.set_x_ticks(None, &[], &[]);
		}
		for category in categories.values_mut() {
			let name: &str = if show_names { &category.category_name } else { "" };
			let hours = category.time_impact as f64 / 60.0 / 60.0;
			let hours = if show_hours { format!(" {:.0}", hours) } else { "".to_string() };
			let caption = format!("{}{}", name, hours);
			axes.lines(&x_coord,
				&category.points,
				&[Caption(&caption),
					Color(&category.color),
					PointSize(1.0),
					PointSymbol('*')
				],
			);
		}
	}
	figure.echo_to_file(dirs.cache_dir().join("gnuplot").to_str().unwrap());
	figure.show();
}

fn ensure_file(filename: &PathBuf, content: &str) {
	if Path::new(&filename).exists() == false {
		let mut file = OpenOptions::new().create(true).write(true).open(filename).unwrap();
		file.write_all(content.as_bytes()).unwrap();
	}
}

fn get_category(desktop_number: u32, window_name: &str, dirs: &ProjectDirs) -> String {
	{
		// TODO: allow ignoring xprintidle
		let idle_time = Command::new("xprintidle").output().unwrap();
		assert!(idle_time.status.success());
		let idle_time = String::from_utf8(idle_time.stdout).unwrap();
		let idle_time = idle_time.trim().parse::<u64>().unwrap() / 1000;
		if idle_time > 60 * 3 { // 3min
			eprintln!("idle time: {}", idle_time);
			return "skip".to_string();
		}
	}
	let window_name = window_name.to_lowercase();
	let window_name = window_name.as_str();
	if Path::new(&dirs.config_dir().join("category_decider")).exists() {
		let child = Command::new(dirs.config_dir().join("category_decider")).output().unwrap();
		assert!(child.status.success());
		String::from_utf8(child.stdout).unwrap()
	} else {
		let rules_file = File::open(dirs.config_dir().join(RULES_FILE_NAME)).unwrap();
		let rules_file = BufReader::new(rules_file);

		for line in rules_file.lines() {
			let line = line.unwrap();
			if line.starts_with("#") || line.is_empty() {
				continue;
			}
			let split: Vec<&str> = line.splitn(2, ' ').collect();
			let category = *split.get(0).unwrap();
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


fn get_window_name_and_desktop() -> (String, u32) {
	let command = Command::new("xdotool")
		.arg("getactivewindow")
		.arg("get_desktop")
		.arg("getwindowname")
		.output().unwrap();
	// eprintln!("command stdout: \n{}\nstderr:\n{}", String::from_utf8_lossy(&command.stdout), String::from_utf8_lossy(&command.stderr));
	assert!(command.status.success(),
		"command failed with stdout:\n{}\nstderr:\n{}",
		String::from_utf8_lossy(&command.stdout),
		String::from_utf8_lossy(&command.stderr)
	);
//	assert!(command.status.success());
	let stdout = String::from_utf8_lossy(&command.stdout);
	let split: Vec<&str> = stdout.split('\n').collect();
	let window_name = split[1].replace("\n", "").as_str().chars()
		.take(WINDOW_MAX_LENGTH).collect::<String>();
	(window_name, split[0].parse::<u32>().unwrap())
}

fn do_save_current(dirs: &ProjectDirs) {
	let (window_name, desktop_number) = get_window_name_and_desktop();
	// eprintln!("We're on desktop {} and our window is {}", desktop_number, window_name);
	std::env::set_var("DESKTOP_NUMBER", desktop_number.to_string());
	std::env::set_var("WINDOW_NAME", &window_name);
	let category = get_category(desktop_number, &window_name, dirs);
	std::env::set_var("CATEGORY", &category);

	let mut file = OpenOptions::new()
		.append(true).create(true)
		.open(dirs.data_local_dir().join(LOG_FILE_NAME)).unwrap();
	let log_line = format!("{} {} {} {}",
		Utc::now().format(DATE_FORMAT),
		category,
		desktop_number,
		window_name);
	eprintln!("logging: {}", log_line);
	file.write_all(log_line.as_bytes()).unwrap();
	file.write_all("\n".as_bytes()).unwrap();
}

#[cfg(target_os = "linux")]
const AUTOSTART_FILE: &'static str = include_str!("../res/linux_autostart.desktop");

#[cfg(target_os = "linux")]
fn add_to_autostart() {
	let bin_path = Path::new(&std::env::args().next().unwrap()).canonicalize().unwrap();
	let file_str = AUTOSTART_FILE.replace("%PATH%", bin_path.to_str().unwrap());
	//let file_path = match std::env::var_os("XDG_CONFIG_HOME") {
	//	Some(var) => var.to_str().unwrap().to_string(),
	//	None => UserDirs::new().unwrap().home_dir().join(".config").to_str().unwrap().to_string()
	//};
	//let file_path = Path::new(&file_path).join("autostart/TimePlot.desktop");
	let file_path = UserDirs::new().unwrap().home_dir().join(".config/autostart/TimePlot.desktop");
	ensure_file(&file_path, &file_str);
}

fn ensure_env(key: &str, value: &str) {
	if std::env::var_os(key).is_none() {
		std::env::set_var(key, value);
	}
}

#[cfg(target_os = "linux")]
fn main() {
	eprintln!("Timeplot version {}", env!("CARGO_PKG_VERSION"));
	ensure_env("PATH", "/usr/local/bin:/usr/bin:/bin:/usr/local/sbin");
	ensure_env("DISPLAY", ":0.0");
	ensure_env("XAUTHORITY", UserDirs::new().unwrap().home_dir().join(".Xauthority").to_str().unwrap());

	let dirs = ProjectDirs::from("com.gitlab", "vn971", "timeplot").unwrap();

	eprintln!("Config dir: {}", dirs.config_dir().to_str().unwrap());
	std::fs::create_dir_all(dirs.config_dir()).unwrap();
	eprintln!("Cache dir: {}", dirs.cache_dir().to_str().unwrap());
	std::fs::create_dir_all(dirs.cache_dir()).unwrap();
	eprintln!("Data dir: {}", dirs.data_local_dir().to_str().unwrap());
	std::fs::create_dir_all(dirs.data_local_dir()).unwrap();

	ensure_file(&dirs.config_dir().join(RULES_FILE_NAME), RULES_EXAMPLE);
	let config_path = dirs.config_dir().join(CONFIG_FILE_NAME);
	ensure_file(&config_path, CONFIG_EXAMPLE);

	let locked_file = File::open(dirs.config_dir()).unwrap();
	locked_file.try_lock_exclusive().expect("Another instance of timeplot is already running.");

	add_to_autostart();

	loop {
		let mut conf = config::Config::default();
		conf.merge(config::File::with_name(config_path.to_str().unwrap())).unwrap();

		do_save_current(&dirs);
		do_plot(&dirs, &conf);
		let sleep_min = conf.get_float("main.sleep_minutes").expect(CONFIG_PARSE_ERROR);
		let duration = Duration::from_secs((sleep_min * 60.0) as u64);
		std::thread::sleep(duration);
	}
}
