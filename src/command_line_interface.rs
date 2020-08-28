use lazy_static::lazy_static;
use std::path::PathBuf;
use structopt::clap::AppSettings;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(
    setting = AppSettings::DeriveDisplayOrder,
    setting = AppSettings::UnifiedHelpMessage,
)]
pub struct CLIOptions {
	/// Use an alternative config file.
	/// By default, ~/.config/timeplot/config.toml (or your XDG override)
	#[structopt(short, long, name = "CONFIG_FILE", parse(from_os_str))]
	pub config: Option<PathBuf>,
}

lazy_static! {
	pub static ref PARSED: CLIOptions = CLIOptions::from_args();
}
