#[global_allocator]
static GLOBAL: std::alloc::System = std::alloc::System;

mod autostart;
mod file_operations;
mod plotting;
mod timeplot_constants;

extern crate chrono;
extern crate config;
extern crate directories;
extern crate env_logger;
extern crate fs2;
extern crate gnuplot;
extern crate open;
#[macro_use]
extern crate log;
#[cfg(target_os = "windows")]
extern crate user32;
#[cfg(target_os = "windows")]
extern crate winapi;

use crate::timeplot_constants::CONFIG_PARSE_ERROR;
use crate::timeplot_constants::DATE_FORMAT;
use crate::timeplot_constants::LOG_FILE_NAME;
use chrono::prelude::*;
use config::Config;
use directories::ProjectDirs;
use directories::UserDirs;
use env_logger::Env;
use fs2::FileExt;
use std::env;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

const WINDOW_MAX_LENGTH: usize = 200;
const RULES_FILE_NAME: &str = "rules_simple.txt";
#[cfg(target_os = "macos")]
const MAC_SCRIPT_NAME: &str = "get_title.scpt";

#[cfg(not(target_os = "windows"))]
fn log_command_failure(child: &std::process::Output) {
	if !child.status.success() {
		warn!(
			"command failed with exit code {:?}\nStderr: {}\nStdout: {}",
			child.status.code(),
			String::from_utf8_lossy(&child.stderr),
			String::from_utf8_lossy(&child.stdout),
		);
	}
}

fn get_category(activity_info: &WindowActivityInformation, dirs: &ProjectDirs) -> String {
	let window_name = activity_info.window_name.to_lowercase();
	let rules_file = dirs.config_dir().join(RULES_FILE_NAME);
	let rules_file = File::open(&rules_file)
		.unwrap_or_else(|err| panic!("Failed to open rules file {:?}, {}", rules_file, err));
	let rules_file = BufReader::new(rules_file);

	for (line_number, line) in rules_file.lines().enumerate() {
		let line = line
			.unwrap_or_else(|err| panic!("failed to read rules on line {}, {}", line_number, err));
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
		.output()
		.expect("window title extraction script failed to launch");
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
		.output()
		.expect("ERROR: command not found: xdotool");
	log_command_failure(&command);

	let idle_time = match Command::new("xprintidle").output() {
		Err(err) => {
			warn!(
				"Failed to run xprintidle. Assuming window is not idle. Error: {}",
				err
			);
			0
		}
		Ok(output) => {
			let output = String::from_utf8_lossy(&output.stdout);
			let output = output.trim();
			output.parse::<u32>().unwrap_or_else(|err| {
				warn!(
					"Failed to parse xprintidle output '{}', error is: {}",
					output, err
				);
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
			warn!(
				"Empty command for category {}, better remove command altogether",
				category
			);
			return;
		}
	};
	let child = Command::new(executable_name)
		.args(&category_command[1..])
		.env("CATEGORY", category)
		.env("WINDOW_NAME", window_name)
		.output();
	match child {
		Err(err) => warn!(
			"Failed to run command '{}' for category {}, error is {}",
			executable_name, category, err
		),
		Ok(child) => {
			if !child.status.success() {
				warn!(
					"Non-zero exit code for category {}, command {:?}",
					category, &category_command
				)
			}
		}
	}
}

fn do_save_current(dirs: &ProjectDirs, image_dir: &PathBuf, conf: &Config) {
	let mut activity_info = get_window_activity_info(dirs);
	activity_info.window_name = activity_info.window_name.trim().replace("\n", " ");
	if activity_info.idle_seconds > 60 * 3 {
		info!(
			"skipping log due to inactivity time: {}sec, {}",
			activity_info.idle_seconds, activity_info.window_name
		);
		return;
	}
	let category = get_category(&activity_info, dirs);
	run_category_command(conf, &category, &activity_info.window_name);

	let file_path = image_dir.join(LOG_FILE_NAME);
	let mut file = OpenOptions::new()
		.append(true)
		.create(true)
		.open(&file_path)
		.unwrap_or_else(|err| panic!("failed to open log file {:?}, {}", file_path, err));
	let log_line = format!(
		"{} {} {}",
		Utc::now().format(DATE_FORMAT),
		category,
		activity_info
			.window_name
			.chars()
			.take(WINDOW_MAX_LENGTH)
			.collect::<String>()
	); //todo vn
	info!("logging: {}", log_line);
	file.write_all(log_line.as_bytes())
		.unwrap_or_else(|err| panic!("Failed to write to log file {:?}, {}", file_path, err));
	file.write_all("\n".as_bytes()).unwrap_or_else(|err| {
		panic!(
			"Failed to write newline to log file {:?}, {}",
			file_path, err
		)
	});
}

#[cfg(target_os = "windows")]
pub fn prepare_scripts(_: &ProjectDirs) {}
#[cfg(target_os = "macos")]
pub fn prepare_scripts(dirs: &ProjectDirs) {
	use std::os::unix::fs::PermissionsExt;
	let path = dirs.config_dir().join(MAC_SCRIPT_NAME);
	file_operations::ensure_file(&path, &include_str!("../res/macos_get_title.scpt"));
	fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
		.unwrap_or_else(|err| panic!("failed to set permissions for {:?}, {}", path, err));
}
#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
pub fn prepare_scripts(_: &ProjectDirs) {}

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
	debug!(
		"{} version {}",
		env!("CARGO_PKG_NAME"),
		env!("CARGO_PKG_VERSION")
	);

	let user_dirs = UserDirs::new().expect("failed to calculate user dirs (like ~)");
	let dirs = ProjectDirs::from("com.gitlab", "vn971", "timeplot")
		.expect("failed to calculate ProjectDirs");
	let image_dir = user_dirs
		.picture_dir()
		.filter(|f| f.exists())
		.map(|f| f.join("timeplot"))
		.unwrap_or_else(|| dirs.data_local_dir().to_path_buf());

	default_env("PATH", "/usr/local/bin:/usr/bin:/bin:/usr/local/sbin");
	default_env("DISPLAY", ":0.0");
	default_env(
		"XAUTHORITY",
		user_dirs.home_dir().join(".Xauthority").to_str().unwrap(),
	);

	info!("Config dir: {}", dirs.config_dir().to_str().unwrap());
	fs::create_dir_all(dirs.config_dir()).expect("Failed to create config dir");
	info!("Image dir: {}", image_dir.to_str().unwrap());
	fs::create_dir_all(&image_dir).expect("Failed to create image dir");

	file_operations::ensure_file(
		&dirs.config_dir().join(RULES_FILE_NAME),
		&include_str!("../res/example_rules_simple.txt"),
	);
	let config_path = dirs.config_dir().join("config.toml");
	file_operations::ensure_file(&config_path, include_str!("../res/example_config.toml"));

	let mut conf = config::Config::default();
	conf.merge(config::File::with_name(config_path.to_str().unwrap()))
		.expect("Failed to read config file");

	if conf
		.get_bool("beginner.create_autostart_entry")
		.unwrap_or(false)
	{
		autostart::add_to_autostart();
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

	let locked_file =
		File::open(dirs.config_dir()).expect("failed to open config directory for locking");
	if let Err(err) = locked_file.try_lock_exclusive() {
		eprintln!(
			"Another instance of timeplot is already running, could not acquire lock: {}",
			err
		);
		std::process::exit(1)
	}

	loop {
		if let Err(err) = conf.refresh() {
			warn!("Failed to refresh configuration, {}", err);
		}
		do_save_current(&dirs, &image_dir, &conf);
		plotting::do_plot(&image_dir, &conf);
		let sleep_min = conf
			.get_float("main.sleep_minutes")
			.expect(CONFIG_PARSE_ERROR);
		std::thread::sleep(Duration::from_secs((sleep_min * 60.0) as u64));
	}
}
