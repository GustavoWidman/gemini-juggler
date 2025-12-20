use std::path::PathBuf;

use clap::Parser;
use log::LevelFilter;

#[derive(Parser, Debug)]
#[command(name = "gemini-juggler")]
pub struct Args {
    /// Path to the configuration file
    #[arg(short, long, value_name = "FILE", default_value = "config.toml")]
    pub config: PathBuf,

    /// Sets the logger's verbosity level
    #[arg(short, long, value_name = "VERBOSITY", default_value_t = LevelFilter::Info)]
    pub verbosity: LevelFilter,
}
