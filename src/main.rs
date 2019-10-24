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

use chrono::prelude::*;
use config::Config;
use directories::ProjectDirs;
use directories::UserDirs;
use env_logger::Env;
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
use std::ops::Not;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
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
	let log_file = image_dir.join(LOG_FILE_NAME);
	let log_file = File::open(&log_file).unwrap_or_else(|err| panic!("Failed to open log file {:?}, {}", log_file, err));
	let mut log_file = BufReader::new(log_file);

	// seek forward until we reach recent entries
	let mut pos = 0;
	loop {
		pos += FILE_SEEK;
		log_file.seek(SeekFrom::Start(pos)).unwrap_or_else(|err| panic!("{}:{} seeking failed, {}", file!(), line!(), err));
		log_file.read_until(b'\n', &mut Vec::new()).unwrap_or_else(|err| panic!("{}:{}, failed reading till first newline {}", file!(), line!(), err));
		log_file.read_until(b'\n', &mut Vec::new()).unwrap_or_else(|err| panic!("{}:{}, failed reading till second newline {}", file!(), line!(), err));
		let mut line = String::new();
		log_file.read_line(&mut line).expect("Failed to read line from log (file seeking to find latest entries)");
		if line.is_empty() || parse_log_line(&line).epoch_seconds > min_time {
			pos -= FILE_SEEK;
			log_file.seek(SeekFrom::Start(pos)).expect("Failed to seek log file (to find latest entries)");
			if pos > 0 {
				log_file.read_until(b'\n', &mut Vec::new()).unwrap_or_else(|err| panic!("{}:{}, {}", file!(), line!(), err));
			}
			break;
		}
	}

	let mut lines: Vec<_> = log_file.lines().map(|l| parse_log_line(&l.expect("failed to get log line"))).collect();
	lines.reverse();

	let mut categories: HashMap<&str, CategoryData> = HashMap::new();
	// TODO: pre-fill categories to have deterministic order

	let mut last_time = time_now;
	for line in lines.iter_mut() {
		if line.epoch_seconds < min_time { continue; }
		if !conf.get_bool(&format!("category.{}.hide", &line.category)).unwrap_or(false)
			&& categories.contains_key(line.category.as_str()).not() {
			let is_empty = categories.is_empty();
			categories.insert(&line.category, CategoryData {
				category_name: line.category.to_string(),
				color: conf.get_str(&format!("category.{}.color", &line.category)).unwrap_or_else(|_| "black".to_string()).to_string(),
				time_impact: 0,
				values: if is_empty { Vec::new() } else { vec![0.0] },
				keys: if is_empty { Vec::new() } else { vec![last_time] },
			});
		}
		line.epoch_seconds = min(line.epoch_seconds, last_time);
		while last_time > line.epoch_seconds + sleep_seconds {
			last_time -= sleep_seconds;
			for category in categories.values_mut() {
				let last = category.values.last().cloned();
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
			let old_value = category.values.last().cloned().unwrap_or(latest);
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
			axes.set_x_ticks(Some((Fix(1.0), 0)), &[OnAxis(false), Inward(false), Mirror(false)], &[]);
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
	if Path::new(&filename).exists().not() {
		let mut file = OpenOptions::new().create(true).write(true).open(filename).unwrap_or_else(|err| panic!("Failed to open or create file {:?}, {}", filename, err));
		file.write_all(content.as_bytes()).unwrap_or_else(|err| panic!("failed to write to file {:?}, {}", filename, err));
	}
}

#[cfg(not(target_os = "windows"))]
fn log_command_failure(child: &std::process::Output) {
	if child.status.success().not() {
		warn!("command failed with exit code {:?}\nStderr: {}\nStdout: {}",
			child.status.code(),
			String::from_utf8_lossy(&child.stderr),
			String::from_utf8_lossy(&child.stdout),
		);
	}
}


fn get_category(activity_info: &WindowActivityInformation, dirs: &ProjectDirs) -> String {
	let window_name = activity_info.window_name.to_lowercase();
	let rules_file = dirs.config_dir().join(RULES_FILE_NAME);
	let rules_file = File::open(&rules_file).unwrap_or_else(|err| panic!("Failed to open rules file {:?}, {}", rules_file, err));
	let rules_file = BufReader::new(rules_file);

	for (line_number, line) in rules_file.lines().enumerate() {
		let line = line.unwrap_or_else(|err| panic!("failed to read rules on line {}, {}", line_number, err));
		let line = line.trim_start();
		if line.starts_with('#') || line.is_empty() {
			continue;
		}
		let split: Vec<&str> = line.splitn(2, ' ').collect();
		let category = split[0];
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
	let command = Command::new(dirs.config_dir().join(MAC_SCRIPT_NAME))
		.output().expect("window title extraction script failed to launch");
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
		window_name: String::from_utf16_lossy(&vec),
		idle_seconds: 0,
	}
}
#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
fn get_window_activity_info(_: &ProjectDirs) -> WindowActivityInformation {
	let command = Command::new("xdotool")
		.arg("getactivewindow")
		.arg("getwindowname")
		.output().expect("ERROR: command not found: xdotool");
	log_command_failure(&command);

	let idle_time = match Command::new("xprintidle").output() {
		Err(err) => {
			warn!("Failed to run xprintidle. Assuming window is not idle. Error: {}", err);
			0
		},
		Ok(output) => {
			let output = String::from_utf8_lossy(&output.stdout);
			let output = output.trim();
			output.parse::<u32>().unwrap_or_else(|err| {
				warn!("Failed to parse xprintidle output '{}', error is: {}", output, err);
				0
			}) / 1000
		}
	};

	WindowActivityInformation {
		window_name: String::from_utf8_lossy(&command.stdout).to_string(),
		idle_seconds: idle_time,
	}
}

fn run_category_command(conf: &Config, category: &str, window_name: &str) {
	let conf_key = format!("category.{}.command", category);
	let category_command = conf.get::<Vec<String>>(&conf_key);

	let category_command = match category_command {
		Ok(command) => command,
		Err(_) => return, // it's OK to not define a command for a category
	};
	let executable_name = match category_command.first() {
		Some(executable) => executable,
		None => {
			warn!("Empty command for category {}, better remove command altogether", category);
			return;
		}
	};
	let child = Command::new(executable_name).args(&category_command[1..])
		.env("CATEGORY", category).env("WINDOW_NAME", window_name).output();
	match child {
		Err(err) =>
			warn!("Failed to run command '{}' for category {}, error is {}", executable_name, category, err),
		Ok(child) => if !child.status.success() {
			warn!("Non-zero exit code for category {}, command {:?}", category, &category_command)
		},
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

	let file_path = image_dir.join(LOG_FILE_NAME);
	let mut file = OpenOptions::new()
		.append(true).create(true)
		.open(&file_path).unwrap_or_else(|err| panic!("failed to open log file {:?}, {}", file_path, err));
	let log_line = format!("{} {} {}",
		Utc::now().format(DATE_FORMAT),
		category,
		activity_info.window_name.chars().take(WINDOW_MAX_LENGTH).collect::<String>());//todo vn
	info!("logging: {}", log_line);
	file.write_all(log_line.as_bytes()).unwrap_or_else(|err| panic!("Failed to write to log file {:?}, {}", file_path, err));
	file.write_all("\n".as_bytes()).unwrap_or_else(|err| panic!("Failed to write newline to log file {:?}, {}", file_path, err));
}

#[cfg(target_os = "linux")]
fn add_to_autostart() {
	let executable_name = env::current_exe().expect("failed to get current executable name");
	let executable_name = executable_name.to_str().unwrap_or_else(|| panic!("failet to read executable name '{:?}' to string", executable_name));
	let xdg_desktop = include_str!("../res/linux_autostart.desktop");
	let xdg_desktop = xdg_desktop.replace("%PATH%", executable_name);
	let file_path = UserDirs::new().expect("failed to calculate user dirs")
		.home_dir().join(".config/autostart/TimePlot.desktop");
	ensure_file(&file_path, &xdg_desktop);
}
#[cfg(not(target_os = "linux"))]
fn add_to_autostart() {}


#[cfg(target_os = "windows")]
pub fn prepare_scripts(_: &ProjectDirs) {
}
#[cfg(target_os = "macos")]
pub fn prepare_scripts(dirs: &ProjectDirs) { // main_prepare_files
	use std::os::unix::fs::PermissionsExt;
	let path = dirs.config_dir().join(MAC_SCRIPT_NAME);
	ensure_file(&path, &include_str!("../res/macos_get_title.scpt"));
	fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
		.unwrap_or_else(|err| panic!("failed to set permissions for {:?}, {}", path, err));
}
#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
pub fn prepare_scripts(_: &ProjectDirs) {
}


fn default_env(key: &str, value: &str) {
	if env::var_os(key).is_none() {
		env::set_var(key, value);
	}
}

fn main() {
	default_env("RUST_BACKTRACE", "1"); // if it wasn't set to "0" explicitly, set it to 1.
	default_env("RUST_LOG", "info");
	env_logger::Builder::from_env(Env::default().filter_or("LOG_LEVEL", "info"))
		.format(|buf, record| {
			writeln!(
				buf,
				"{} [{}] - {}",
				Utc::now().format("%Y-%m-%d %H:%M:%S"),
				record.level(),
				record.args()
			)
		})
		.init();
	debug!("{} version {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

	let user_dirs = UserDirs::new().expect("failed to calculate user dirs (like ~)");
	let dirs = ProjectDirs::from("com.gitlab", "vn971", "timeplot").expect("failed to calculate ProjectDirs");
	let image_dir = user_dirs.picture_dir().filter(|f| f.exists())
		.map(|f| f.join("timeplot"))
		.unwrap_or_else(|| dirs.data_local_dir().to_path_buf());

	default_env("PATH", "/usr/local/bin:/usr/bin:/bin:/usr/local/sbin");
	default_env("DISPLAY", ":0.0");
	default_env("XAUTHORITY", user_dirs.home_dir().join(".Xauthority").to_str().unwrap());

	info!("Config dir: {}", dirs.config_dir().to_str().unwrap());
	fs::create_dir_all(dirs.config_dir()).expect("Failed to create config dir");
	info!("Image dir: {}", image_dir.to_str().unwrap());
	fs::create_dir_all(&image_dir).expect("Failed to create image dir");

	ensure_file(&dirs.config_dir().join(RULES_FILE_NAME), &include_str!("../res/example_rules_simple.txt"));
	let config_path = dirs.config_dir().join("config.toml");
	ensure_file(&config_path, include_str!("../res/example_config.toml"));

	let mut conf = config::Config::default();
	conf.merge(config::File::with_name(config_path.to_str().unwrap())).expect("Failed to read config file");

	if conf.get_bool("beginner.create_autostart_entry").unwrap_or(false) {
		add_to_autostart();
	}
	if conf.get_bool("beginner.show_directories").unwrap_or(true) {
		if let Err(err) = open::that(dirs.config_dir()) {
			eprintln!("Debug: failed to `open` config directory, {}", err);
		}
		if let Err(err) = open::that(&image_dir) {
			eprintln!("Debug: failed to `open` image directory, {}", err);
		}
	}
	prepare_scripts(&dirs);

	let locked_file = File::open(dirs.config_dir()).expect("failed to open config directory for locking");
	locked_file.try_lock_exclusive().expect("Another instance of timeplot is already running.");

	loop {
		if let Err(err) = conf.refresh() {
			warn!("Failed to refresh configuration, {}", err);
		}
		do_save_current(&dirs, &image_dir, &conf);
		do_plot(&image_dir, &conf);
		let sleep_min = conf.get_float("main.sleep_minutes").expect(CONFIG_PARSE_ERROR);
		std::thread::sleep(Duration::from_secs((sleep_min * 60.0) as u64));
	}
}
