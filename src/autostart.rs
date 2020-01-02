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

use std::env;

#[cfg(target_os = "linux")]
pub fn add_to_autostart() {
	use crate::file_operations;
	let xdg_desktop = include_str!("../res/linux_autostart.desktop");
	let xdg_desktop = xdg_desktop.replace("%PATH%", &_executable_name());
	let file_path = directories::UserDirs::new()
		.expect("failed to calculate user dirs")
		.home_dir()
		.join(".config/autostart/TimePlot.desktop");
	file_operations::ensure_file(&file_path, &xdg_desktop);
}

#[cfg(target_os = "macos")]
pub fn add_to_autostart() {
	use crate::file_operations;
	let plist = include_str!("../res/macos_timeplot.plist");
	let exe = _executable_name()
		.replace("\"", "&quot;")
		.replace("'", "&apos;")
		.replace("<", "&lt;")
		.replace(">", "&gt;")
		.replace("&", "&amp;");
	let plist = plist.replace("%PATH%", &exe);
	let file_path = directories::UserDirs::new()
		.expect("failed to calculate user dirs")
		.home_dir()
		.join("Library/LaunchAgents/timeplot.plist");
	file_operations::ensure_file(&file_path, &plist);

	std::process::Command::new("launchctl")
		.arg("load")
		.arg("-w")
		.arg(file_path)
		.output()
		.expect("failed to execute launchctl");
}

#[cfg(all(not(target_os = "linux"), not(target_os = "macos")))]
pub fn add_to_autostart() {}

fn _executable_name() -> String {
	let exe = env::current_exe()
		.unwrap_or_else(|err| panic!("Failed to get current executable, {}", err));
	let exe = exe
		.to_str()
		.unwrap_or_else(|| panic!("Failed to parse executable name to string"));
	exe.to_string()
}
