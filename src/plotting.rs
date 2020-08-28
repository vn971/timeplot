extern crate chrono;
extern crate config;
extern crate directories;
extern crate env_logger;
extern crate fs2;
extern crate gnuplot;
extern crate open;

use crate::timeplot_constants::CONFIG_PARSE_ERROR;
use crate::timeplot_constants::DATE_FORMAT;
use crate::timeplot_constants::FILE_SEEK;
use crate::timeplot_constants::LOG_FILE_NAME;
use chrono::prelude::*;
use chrono::Duration;
use config::Config;
use log::warn;
use std::cmp::min;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::SeekFrom;
use std::ops::Not;
use std::ops::Sub;
use std::path::PathBuf;

/// The part of log entry that needs to be parsed.
struct LogEntry {
	epoch_seconds: u64,
	category: String,
}

fn parse_log_line(line: &str) -> LogEntry {
	let split: Vec<&str> = line.splitn(3, ' ').collect();
	let parse_error = format!("Failed to parse log entry {}", line);
	let time = Utc
		.datetime_from_str(split.get(0).expect(&parse_error), DATE_FORMAT)
		.expect(&parse_error);
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

pub fn do_plot(image_dir: &PathBuf, conf: &Config) {
	use gnuplot::*;
	let sleep_seconds = conf
		.get_float("main.sleep_minutes")
		.expect(CONFIG_PARSE_ERROR);
	let sleep_seconds = (sleep_seconds * 60.0) as u64;
	let plot_days = conf.get_float("main.plot_days").expect(CONFIG_PARSE_ERROR);
	let smoothing = conf.get_float("graph.smoothing").expect(CONFIG_PARSE_ERROR);
	let smoothing = -plot_days as f32 * smoothing as f32 * 100.0;
	let data_absence_modifier = (sleep_seconds as f32 / smoothing).exp2();

	let time_now = Utc::now();
	let min_time = time_now.sub(Duration::seconds((plot_days * 60.0 * 60.0 * 24.0) as i64));
	let min_time = if conf.get_bool("main.plot_truncate_to_5am").unwrap_or(false) {
		let mut min_time = min_time.with_timezone(&chrono::Local);
		min_time = min_time.with_hour(5).unwrap();
		min_time = min_time.with_minute(0).unwrap();
		min_time = min_time.with_second(0).unwrap();
		if min_time > time_now {
			min_time = min_time.sub(Duration::days(1));
		};
		min_time.with_timezone(&chrono::Utc)
	} else {
		min_time
	};
	let min_time = min_time.timestamp() as u64;
	let log_file = image_dir.join(LOG_FILE_NAME);
	let log_file = File::open(&log_file)
		.unwrap_or_else(|err| panic!("Failed to open log file {:?}, {}", log_file, err));
	let mut log_file = BufReader::new(log_file);

	// seek forward until we reach recent entries
	let mut pos = 0;
	loop {
		pos += FILE_SEEK;
		log_file
			.seek(SeekFrom::Start(pos))
			.unwrap_or_else(|err| panic!("{}:{} seeking failed, {}", file!(), line!(), err));
		log_file
			.read_until(b'\n', &mut Vec::new())
			.unwrap_or_else(|err| panic!("failed reading till first newline {}", err));
		log_file
			.read_until(b'\n', &mut Vec::new())
			.unwrap_or_else(|err| panic!("failed reading till second newline {}", err));
		let mut line = String::new();
		log_file
			.read_line(&mut line)
			.expect("Failed to read line from log (file seeking to find latest entries)");
		if line.is_empty() || parse_log_line(&line).epoch_seconds > min_time {
			pos -= FILE_SEEK;
			log_file
				.seek(SeekFrom::Start(pos))
				.expect("Failed to seek log file (to find latest entries)");
			if pos > 0 {
				log_file
					.read_until(b'\n', &mut Vec::new())
					.unwrap_or_else(|err| panic!("{}:{}, {}", file!(), line!(), err));
			}
			break;
		}
	}

	let mut lines: Vec<_> = log_file
		.lines()
		.map(|l| parse_log_line(&l.expect("failed to get log line")))
		.collect();
	lines.reverse();

	let mut categories: HashMap<&str, CategoryData> = HashMap::new();
	// TODO: pre-fill categories to have deterministic order

	let mut last_time = time_now.timestamp() as u64;
	for line in lines.iter_mut() {
		if line.epoch_seconds < min_time {
			continue;
		}
		if !conf
			.get_bool(&format!("category.{}.hide", &line.category))
			.unwrap_or(false)
			&& categories.contains_key(line.category.as_str()).not()
		{
			let is_empty = categories.is_empty();
			categories.insert(
				&line.category,
				CategoryData {
					category_name: line.category.to_string(),
					color: conf
						.get_str(&format!("category.{}.color", &line.category))
						.unwrap_or_else(|_| "black".to_string()),
					time_impact: 0,
					values: if is_empty { Vec::new() } else { vec![0.0] },
					keys: if is_empty {
						Vec::new()
					} else {
						vec![last_time]
					},
				},
			);
		}
		line.epoch_seconds = min(line.epoch_seconds, last_time);
		while last_time > line.epoch_seconds + sleep_seconds {
			last_time -= sleep_seconds;
			for category in categories.values_mut() {
				let last = category.values.last().cloned();
				category.keys.push(last_time);
				category
					.values
					.push(last.unwrap_or(0.0) * data_absence_modifier);
			}
		}
		let time_diff = last_time - line.epoch_seconds;
		let weight_old = (time_diff as f32 / smoothing).exp2();
		let weight_new = 1.0 - weight_old;
		for category in categories.values_mut() {
			if line.category == category.category_name {
				category.time_impact += min(time_diff, sleep_seconds);
			};
			let latest = if line.category == category.category_name {
				1.0
			} else {
				0.0
			};
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
	let show_date = conf.get_bool("graph.show_date").expect(CONFIG_PARSE_ERROR);
	let show_day_ticks = conf
		.get_bool("graph.show_day_ticks")
		.or_else(|_| conf.get_bool("graph.show_day_labels"))
		.expect(CONFIG_PARSE_ERROR);

	{
		let axes = figure
			.axes2d()
			.set_y_ticks(None, &[], &[])
			.set_border(false, &[], &[])
			.set_y_range(
				Fix(-0.1),
				Fix(conf
					.get_float("graph.height_scale")
					.expect(CONFIG_PARSE_ERROR)),
			);
		if show_date {
			axes.set_x_label(
				&Local::now()
					.naive_local()
					.format("created at: %Y-%m-%d %H:%M")
					.to_string(),
				&[],
			);
		}
		if show_day_ticks {
			axes.set_x_ticks(
				Some((Fix(1.0), 0)),
				&[OnAxis(false), Inward(false), Mirror(false)],
				&[],
			);
		} else {
			axes.set_x_ticks(None, &[], &[]);
		}
		let mut categories: Vec<_> = categories.values().collect();
		categories.sort_unstable_by(|a, b| a.time_impact.cmp(&b.time_impact));
		for category in categories {
			let minutes = (category.time_impact as f64 / 60.0).floor() as i64;
			let hours = format!("{}:{:02}", minutes / 60, minutes % 60);
			let caption = label_format
				.replace("%hours%", &hours)
				.replace("%category%", &category.category_name);
			let day_starts_at_00 = conf.get_bool("graph.day_starts_at_00").unwrap_or(true);
			let time_now = if day_starts_at_00 {
				Utc::now().date().and_hms(0, 0, 0).timestamp_millis() / 1000
			} else {
				Utc::now().timestamp_millis() / 1000
			};
			let x_coord: Vec<_> = category
				.keys
				.iter()
				.map(|x| (*x as f64 - time_now as f64) / 60.0 / 60.0 / 24.0)
				.collect();
			axes.lines(
				&x_coord,
				&category.values,
				&[
					Caption(&caption),
					Color(&category.color),
					PointSize(1.0),
					PointSymbol('*'),
				],
			);
		}
	}
	let size_suffix = if size_override.is_empty() {
		"".to_string()
	} else {
		format!(" size {}", size_override)
	};
	figure.set_terminal(
		&format!("svg{}", size_suffix),
		image_dir.join("image.svg").to_str().unwrap(),
	);
	if let Err(err) = figure.show() {
		warn!("Failed to plot svg image, {}", err);
	};
	figure.set_terminal(
		&format!("pngcairo{}", size_suffix),
		image_dir.join("image.png").to_str().unwrap(),
	);
	if let Err(err) = figure.show() {
		warn!("Failed to plot png image, {}", err);
	};
	figure.close();
}
