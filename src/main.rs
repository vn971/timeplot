#[global_allocator]
static GLOBAL: std::alloc::System = std::alloc::System;

extern crate chrono;
extern crate config;
extern crate directories;
extern crate env_logger;
extern crate fs2;
extern crate gnuplot;
extern crate open;
#[macro_use] extern crate log;
#[cfg(target_os = "windows")] extern crate user32;
#[cfg(target_os = "windows")] extern crate winapi;
#[cfg(target_os = "macos")] use std::os::unix::fs::PermissionsExt;

use chrono::prelude::*;
use config::Config;
use directories::ProjectDirs;
use directories::UserDirs;
use fs2::FileExt;
use std::cmp::min;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::fs::OpenOptions;
use std::fs;
use std::io::BufReader;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;
use std::time::Duration;

const WINDOW_MAX_LENGTH: usize = 200;
const FILE_SEEK: u64 = 100_000;
const DATE_FORMAT: &str = "%Y-%m-%d_%H:%M";
const LOG_FILE_NAME: &str = "log.log";
const RULES_FILE_NAME: &str = "rules_simple.txt";
const CONFIG_PARSE_ERROR: &str = "Failed to parse config file. Consider removing/renaming it so it'll be recreated.";
#[cfg(target_os = "macos")] const MAC_SCRIPT_NAME:&str = "get_title.scpt";

/// The part of log entry that needs to be parsed.
struct LogEntry {
	epoch_seconds: u64,
	category: String,
}

fn parse_log_line(line: &str) -> LogEntry {
	let split: Vec<&str> = line.splitn(3, ' ').collect();
	let parse_error = format!("Failed to parse log entry {}", line);
	let time = Utc.datetime_from_str(split.get(0).expect(&parse_error), DATE_FORMAT).expect(&parse_error);
	LogEntry {
		epoch_seconds: time.timestamp_millis() as u64 / 1000,
		category: (*split.get(1).expect(&parse_error)).to_string(),
	}
}


struct CategoryData {
	category_name: String,
	color: String,
	time_impact: u64,
	keys: Vec<u64>,
	values: Vec<f32>,
}

fn do_plot(image_dir: &PathBuf, conf: &Config) {
	use gnuplot::*;
	let sleep_seconds = conf.get_float("main.sleep_minutes").expect(CONFIG_PARSE_ERROR);
	let sleep_seconds = (sleep_seconds * 60.0) as u64;
	let plot_days = conf.get_float("main.plot_days").expect(CONFIG_PARSE_ERROR);
	let smoothing = conf.get_float("graph.smoothing").expect(CONFIG_PARSE_ERROR);
	let smoothing = -plot_days as f32 * smoothing as f32 * 100.0;
	let data_absence_modifier = (sleep_seconds as f32 / smoothing).exp2();

	let time_now = Utc::now().timestamp_millis() as u64 / 1000;
	let min_time = time_now - (plot_days * 60.0 * 60.0 * 24.0) as u64;
	let log_file = File::open(image_dir.join(LOG_FILE_NAME)).unwrap();
	let mut log_file = BufReader::new(log_file);

	// seek forward until we reach recent entries
	let mut pos = 0;
	loop {
		pos += FILE_SEEK;
		log_file.seek(SeekFrom::Start(pos)).unwrap();
		let mut line = String::new();
		log_file.read_line(&mut String::new()).unwrap();
		log_file.read_line(&mut line).unwrap();
		if line.is_empty() || parse_log_line(&line).epoch_seconds > min_time {
			pos -= FILE_SEEK;
			log_file.seek(SeekFrom::Start(pos)).unwrap();
			if pos > 0 { log_file.read_line(&mut String::new()).unwrap(); }
			break;
		}
	}

	let mut lines: Vec<_> = log_file.lines().map(|l| parse_log_line(&l.unwrap())).collect();
	lines.reverse();

	let mut categories: HashMap<&str, CategoryData> = HashMap::new();
	// TODO: pre-fill categories to have deterministic order

	let mut last_time = time_now;
	for line in lines.iter_mut() {
		if line.epoch_seconds < min_time { continue; }
		if !conf.get_bool(&format!("category.{}.hide", &line.category)).unwrap_or(false)
			&& categories.contains_key(line.category.as_str()) == false {
			let is_empty = categories.is_empty();
			categories.insert(&line.category, CategoryData {
				category_name: line.category.to_string(),
				color: conf.get_str(&format!("category.{}.color", &line.category)).unwrap_or("black".to_string()).to_string(),
				time_impact: 0,
				values: if is_empty { Vec::new() } else { vec![0.0] },
				keys: if is_empty { Vec::new() } else { vec![last_time] },
			});
		}
		line.epoch_seconds = min(line.epoch_seconds, last_time);
		while last_time > line.epoch_seconds + sleep_seconds {
			last_time -= sleep_seconds;
			for category in categories.values_mut() {
				let last = category.values.last().map(|x| *x);
				category.keys.push(last_time);
				category.values.push(last.unwrap_or(0.0) * data_absence_modifier);
			}
		}
		let time_diff = last_time - line.epoch_seconds;
		let weight_old = (time_diff as f32 / smoothing).exp2();
		let weight_new = 1.0 - weight_old;
		for category in categories.values_mut() {
			if line.category == category.category_name {
				category.time_impact += min(time_diff, sleep_seconds);
			};
			let latest = if line.category == category.category_name { 1.0 } else { 0.0 };
			let old_value = category.values.last().map(|x| *x).unwrap_or(latest);
			let new_value = Some(latest * weight_new + old_value * weight_old);
			category.keys.push(line.epoch_seconds);
			category.values.push(new_value.unwrap_or(0.0));
		}
		last_time = line.epoch_seconds;
	}

	let mut figure = Figure::new();

	let size_override = conf.get_str("graph.size").expect(CONFIG_PARSE_ERROR);
	let size_override = size_override.trim();
	let label_format = conf.get_str("graph.line_format").expect(CONFIG_PARSE_ERROR);
	let show_days = conf.get_bool("graph.show_day_labels").expect(CONFIG_PARSE_ERROR);
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
		let mut categories: Vec<_> = categories.values().collect();
		categories.sort_unstable_by(|a, b| a.time_impact.cmp(&b.time_impact));
		for category in categories {
			let hours = format!("{:.0}", category.time_impact as f64 / 60.0 / 60.0);
			let caption = label_format.replace("%hours%", &hours)
				.replace("%category%", &category.category_name);
			let day_starts_at_00 = conf.get_bool("graph.day_starts_at_00").unwrap_or(true);
			let time_now = if day_starts_at_00 {
				Utc::now().date().and_hms(0, 0, 0).timestamp_millis() / 1000
			} else {
				Utc::now().timestamp_millis() / 1000
			};
			let x_coord: Vec<_> = category.keys.iter().map(|x|
				(*x as f64 - time_now as f64) / 60.0 / 60.0 / 24.0
			).collect();
			axes.lines(&x_coord, &category.values, &[
				Caption(&caption),
				Color(&category.color),
				PointSize(1.0),
				PointSymbol('*')
			]);
		}
	}
	let size_suffix = if size_override.is_empty() {
		"".to_string()
	} else {
		format!(" size {}", size_override)
	};
	figure.set_terminal(&format!("svg{}", size_suffix), image_dir.join("image.svg").to_str().unwrap());
	figure.show();
	figure.set_terminal(&format!("pngcairo{}", size_suffix), image_dir.join("image.png").to_str().unwrap());
	figure.show();
}


fn ensure_file(filename: &PathBuf, content: &str) {
	if Path::new(&filename).exists() == false {
		let mut file = OpenOptions::new().create(true).write(true).open(filename).unwrap();
		file.write_all(content.as_bytes()).unwrap();
	}
}

fn log_command_failure(child: &Output) {
	if child.status.success() == false {
		warn!("command failed with exit code {:?}\nStderr: {}\nStdout: {}",
			child.status.code(),
			String::from_utf8_lossy(&child.stderr),
			String::from_utf8_lossy(&child.stdout),
		);
	}
}


fn get_category(activity_info: &WindowActivityInformation, dirs: &ProjectDirs) -> String {
	let window_name = activity_info.window_name.to_lowercase();
	let rules_file = File::open(dirs.config_dir().join(RULES_FILE_NAME)).unwrap();
	let rules_file = BufReader::new(rules_file);

	for line in rules_file.lines() {
		let line = line.unwrap();
		if line.starts_with("#") || line.is_empty() {
			continue;
		}
		let split: Vec<&str> = line.trim_left().splitn(2, ' ').collect();
		let category = *split.get(0).unwrap();
		let window_pattern = *split.get(1).unwrap_or(&"");
		let window_pattern = window_pattern.to_lowercase();
		if window_name.contains(&window_pattern) {
			return category.to_string();
		}
	}
	warn!("Could not find any category for: {}", window_name);
	"skip".to_string()
}


struct WindowActivityInformation {
	window_name: String,
	idle_seconds: u32,
}

#[cfg(target_os = "macos")]
fn get_window_activity_info(dirs: &ProjectDirs) -> WindowActivityInformation {
	let command = Command::new(dirs.config_dir().join(MAC_SCRIPT_NAME)).output().unwrap();
	log_command_failure(&command);
	WindowActivityInformation {
		window_name: String::from_utf8_lossy(&command.stdout).to_string(),
		idle_seconds: 0,
	}
}
#[cfg(target_os = "windows")]
fn get_window_activity_info(_: &ProjectDirs) -> WindowActivityInformation {
	let mut vec = Vec::with_capacity(WINDOW_MAX_LENGTH);
	unsafe {
		let hwnd = user32::GetForegroundWindow();
		let err_code = user32::GetWindowTextW(hwnd, vec.as_mut_ptr(), WINDOW_MAX_LENGTH as i32);
		if err_code != 0 {
			warn!("window name extraction failed!");
		}
		assert!(vec.capacity() >= WINDOW_MAX_LENGTH as usize);
		vec.set_len(WINDOW_MAX_LENGTH as usize);
	};
	WindowActivityInformation {
		window_name: String::from_utf16(&vec).unwrap(),
		idle_seconds: 0,
	}
}
#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
fn get_window_activity_info(_: &ProjectDirs) -> WindowActivityInformation {
	let command = Command::new("xdotool")
		.arg("getactivewindow")
		.arg("getwindowname")
		.output().unwrap();
	log_command_failure(&command);

	let idle_time = match Command::new("xprintidle").output() {
		Err(err) => {
			warn!("Failed to run xprintidle. Assuming window is not idle. Error: {}", err);
			0
		},
		Ok(output) => {
			let output = String::from_utf8(output.stdout).unwrap();
			output.trim().parse::<u32>().unwrap() / 1000
		}
	};

	WindowActivityInformation {
		window_name: String::from_utf8_lossy(&command.stdout).to_string(),
		idle_seconds: idle_time,
	}
}

fn run_category_command(conf: &Config, category: &str, window_name: &str) {
	let conf_key = format!("category.{}.command", category);
	let command = conf.get::<Vec<String>>(&conf_key);
	if command.is_err() {
		return;
	}
	let command = command.unwrap();
	if command.is_empty() {
		warn!("Empty command for category {}", category);
		return;
	};
	let left = command.first().unwrap();
	let child = Command::new(left).args(&command[1..])
		.env("CATEGORY", category).env("WINDOW_NAME", window_name).output();
	if child.is_err() {
		warn!("Failed to run command for category {}: `{:?}`", category, command);
	} else if child.unwrap().status.success() == false {
		warn!("Non-zero exit code for category {}, command {:?}", category, &command);
	}
}

fn do_save_current(dirs: &ProjectDirs, image_dir: &PathBuf, conf: &Config) {
	let mut activity_info = get_window_activity_info(dirs);
	activity_info.window_name = activity_info.window_name.trim().replace("\n", " ");
	if activity_info.idle_seconds > 60 * 3 { // 3min
		info!("skipping log due to inactivity time: {}sec, {}",
			activity_info.idle_seconds,
			activity_info.window_name
		);
		return;
	}
	let category = get_category(&activity_info, dirs);
	run_category_command(conf, &category, &activity_info.window_name);

	let mut file = OpenOptions::new()
		.append(true).create(true)
		.open(image_dir.join(LOG_FILE_NAME)).unwrap();
	let log_line = format!("{} {} {}",
		Utc::now().format(DATE_FORMAT),
		category,
		activity_info.window_name.chars().take(WINDOW_MAX_LENGTH).collect::<String>());//todo vn
	info!("logging: {}", log_line);
	file.write_all(log_line.as_bytes()).unwrap();
	file.write_all("\n".as_bytes()).unwrap();
}

#[cfg(target_os = "linux")]
fn add_to_autostart() {
	let xdg_desktop = include_str!("../res/linux_autostart.desktop");
	let file_str = xdg_desktop.replace("%PATH%", env::current_exe().unwrap().to_str().unwrap());
	let file_path = UserDirs::new().unwrap().home_dir().join(".config/autostart/TimePlot.desktop");
	ensure_file(&file_path, &file_str);
}
#[cfg(not(target_os = "linux"))]
fn add_to_autostart() {}


#[cfg(target_os = "windows")]
pub fn prepare_scripts(_: &ProjectDirs) {
}
#[cfg(target_os = "macos")]
pub fn prepare_scripts(dirs: &ProjectDirs) { // main_prepare_files
	let path = dirs.config_dir().join(MAC_SCRIPT_NAME);
	ensure_file(&path, &include_str!("../res/macos_get_title.scpt"));
	let mut perms = fs::metadata(&path).unwrap().permissions();
	perms.set_mode(0o755);
	fs::set_permissions(path, perms).unwrap();
}
#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
pub fn prepare_scripts(_: &ProjectDirs) {
}


fn ensure_env(key: &str, value: &str) {
	if env::var_os(key).is_none() {
		env::set_var(key, value);
	}
}

fn main() {
	ensure_env("RUST_LOG", "info");
	env_logger::Builder::from_default_env()
		.format(|buf, record| writeln!(buf,
			"{} [{}] - {}",
			Utc::now().format("%Y-%m-%d %H:%M:%S"),
			record.level(),
			record.args()
		))
		.init();
	info!("{} version {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

	let user_dirs = UserDirs::new().unwrap();
	let dirs = ProjectDirs::from("com.gitlab", "vn971", "timeplot").unwrap();
	let image_dir = user_dirs.picture_dir().filter(|f| f.exists())
		.map(|f| f.join("timeplot"))
		.unwrap_or(dirs.data_local_dir().to_path_buf());

	ensure_env("PATH", "/usr/local/bin:/usr/bin:/bin:/usr/local/sbin");
	ensure_env("DISPLAY", ":0.0");
	ensure_env("XAUTHORITY", user_dirs.home_dir().join(".Xauthority").to_str().unwrap());

	info!("Config dir: {}", dirs.config_dir().to_str().unwrap());
	fs::create_dir_all(dirs.config_dir()).unwrap();
	info!("Image dir: {}", image_dir.to_str().unwrap());
	fs::create_dir_all(&image_dir).unwrap();

	ensure_file(&dirs.config_dir().join(RULES_FILE_NAME), &include_str!("../res/example_rules_simple.txt"));
	let config_path = dirs.config_dir().join("config.toml");
	ensure_file(&config_path, include_str!("../res/example_config.toml"));

	let mut conf = config::Config::default();
	conf.merge(config::File::with_name(config_path.to_str().unwrap())).unwrap();

	if conf.get_bool("beginner.create_autostart_entry").unwrap_or(false) {
		add_to_autostart();
	}
	if conf.get_bool("beginner.show_directories").unwrap_or(true) {
		open::that(dirs.config_dir()).unwrap();
		open::that(&image_dir).unwrap();
	}
	prepare_scripts(&dirs);

	let locked_file = File::open(dirs.config_dir()).unwrap();
	locked_file.try_lock_exclusive().expect("Another instance of timeplot is already running.");

	loop {
		conf.refresh().unwrap();
		do_save_current(&dirs, &image_dir, &conf);
		do_plot(&image_dir, &conf);
		let sleep_min = conf.get_float("main.sleep_minutes").expect(CONFIG_PARSE_ERROR);
		std::thread::sleep(Duration::from_secs((sleep_min * 60.0) as u64));
	}
}
