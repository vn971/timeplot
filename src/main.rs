extern crate rusqlite;
extern crate time;

//use time::Timespec;
//use rusqlite::Connection;
//use std::process::Output;
use std::process::Command;


//#[derive(Debug)]
//struct Instance {
//    id: u64,
//    app_name: String,
//    desktop_number: u8,
//    created_at: Timespec,
//    idle_time_millis: u64,
//    percent_work: u8,
//    percent_fun: u8,
//    percent_personal: u8,
//    percent_idle: u8,
//}

//fn execute<'a>(str: &'a str) -> &str {
//    let output = Command::new(str).output().unwrap();
//    assert!(output.status.success());
//    let result = String::from_utf8(output.stdout).unwrap();
//    let r2 = result.trim();
//    println!("{}", r2);
//    println!("{}", r2);
//    return "";//"" r2
//}

trait CheckedStdout {
	fn stdout_option(&self) -> Option<Vec<u8>>;
}

//impl CheckedStdout for Output {
//    fn stdout_option(&self) -> Option<Vec<u8>> {
//        if self.status.success() {
//            Some(self.stdout)
//        } else {
//            None
//        }
//    Command::new("true").output().unwrap().stdout_option();
//        // unimplemented!();
//    }
//}

fn main() {
	std::env::set_var("DISPLAY", ":0");
	std::env::set_var("DBUS_SESSION_BUS_ADDRESS", "unix:path=/run/user/1000/bus");
	println!("script launched, args: {:?}", std::env::args().skip(1).collect::<String>());
	//	assert!(command.status.success());
	//	let s = String::from_utf8(command.stdout).unwrap();
	//	let s = s.trim();
	//	println!("xprintidle string: {}", s);
	//	let idle_time: u64 = s.parse().unwrap();
	//	println!("idle_time: {}", idle_time);

	let idle_time: u64 = {
		let idle_time = Command::new("xprintidle").output().unwrap();
		assert!(idle_time.status.success());
		let s = String::from_utf8(idle_time.stdout).unwrap();
		let s = s.trim();
		println!("xprintidle string: {}", s);
		s.parse::<u64>().unwrap()
	};
	println!("idle_time: {}", idle_time);

	let window_name = {
		let command = Command::new("xdotool").arg("getactivewindow").arg("get_desktop").arg("getwindowname").output().unwrap();
		assert!(command.status.success());
		let s = String::from_utf8(command.stdout).unwrap();
		s
//		let ss = s.trim();
	};
	println!("window_name: {}", window_name);

	println!("idle_time: {}", idle_time);


	//	let command = Command::new("xprintidle").output().unwrap();
}

//	val windowInfo: String = "xdotool getactivewindow get_desktop getwindowname".!!
