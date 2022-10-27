use clap::Parser;

use std::path::PathBuf;

/// Minimal mpd terminal client that aims to be simple yet highly configurable
///
/// Homepage: https://github.com/figsoda/mmtc
#[derive(Parser)]
#[command(version)]
pub struct Opts {
    /// Clear query on play
    #[arg(long)]
    pub clear_query_on_play: bool,

    /// Run mpd commands and quit
    ///
    /// See https://mpd.readthedocs.io/en/latest/protocol.html for more information
    #[arg(short = 'C', long, num_args = ..)]
    pub cmd: Option<Vec<String>>,

    /// Cycle through the queue
    #[arg(long)]
    pub cycle: bool,

    /// Don't clear query on play
    #[arg(long, overrides_with = "clear_query_on_play")]
    pub no_clear_query_on_play: bool,

    /// Don't cycle through the queue
    #[arg(long, overrides_with = "cycle")]
    pub no_cycle: bool,

    /// Specify the address of the mpd server
    #[arg(long, value_name = "address")]
    pub address: Option<String>,

    /// Specify the config file
    #[arg(short, long, value_name = "file")]
    pub config: Option<PathBuf>,

    /// The number of lines to jump
    #[arg(long, value_name = "number")]
    pub jump_lines: Option<usize>,

    /// The time to seek in seconds
    #[arg(long, value_name = "number")]
    pub seek_secs: Option<f32>,

    /// The amount of status updates per second
    #[arg(long, value_name = "number")]
    pub ups: Option<f32>,
}
