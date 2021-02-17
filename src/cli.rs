use clap::{AppSettings, Clap};

use std::path::PathBuf;

/// Minimal mpd terminal client that aims to be simple yet highly configurable
///
/// Homepage: https://github.com/figsoda/mmtc
#[derive(Clap)]
#[clap(bin_name = "mmtc", version, global_setting = AppSettings::ColoredHelp)]
pub struct Opts {
    /// Clear query on play
    #[clap(long, multiple_occurrences(true))]
    pub clear_query_on_play: bool,

    /// Cycle through the queue
    #[clap(long, multiple_occurrences(true))]
    pub cycle: bool,

    /// Don't clear query on play
    #[clap(
        long,
        multiple_occurrences(true),
        overrides_with("clear-query-on-play")
    )]
    pub no_clear_query_on_play: bool,

    /// Don't cycle through the queue
    #[clap(long, multiple_occurrences(true), overrides_with("cycle"))]
    pub no_cycle: bool,

    /// Specify the address of the mpd server
    #[clap(long, value_name = "address")]
    pub address: Option<String>,

    /// Specify the config file
    #[clap(short, long, value_name = "file")]
    pub config: Option<PathBuf>,

    /// The number of lines to jump
    #[clap(long, value_name = "number")]
    pub jump_lines: Option<usize>,

    /// The time to seek in seconds
    #[clap(long, value_name = "number")]
    pub seek_secs: Option<f32>,

    /// The amount of status updates per second
    #[clap(long, value_name = "number")]
    pub ups: Option<f32>,
}
