use structopt::{clap::AppSettings, StructOpt};
use tui::widgets::ListState;

use std::{net::SocketAddr, path::PathBuf};

use crate::mpd::{Status, Track};

/// Minimal mpd terminal client that aims to be simple yet highly configurable
///
/// Homepage: https://github.com/figsoda/mmtc
#[derive(StructOpt)]
#[structopt(
    name = "mmtc",
    rename_all = "kebab-case",
    global_setting = AppSettings::ColoredHelp,
)]
pub struct Opts {
    /// Clear query on play
    #[structopt(long)]
    pub clear_query_on_play: bool,

    /// Cycle through the queue
    #[structopt(long)]
    pub cycle: bool,

    /// Don't clear query on play
    #[structopt(long, overrides_with("clear_query_on_play"))]
    pub no_clear_query_on_play: bool,

    /// Don't cycle through the queue
    #[structopt(long, overrides_with("cycle"))]
    pub no_cycle: bool,

    /// Specify the address of the mpd server
    #[structopt(long, value_name = "address")]
    pub address: Option<SocketAddr>,

    /// Specify the config file
    #[structopt(short, long, value_name = "file")]
    pub config: Option<PathBuf>,

    /// The number of lines to jump
    #[structopt(long, value_name = "number")]
    pub jump_lines: Option<usize>,

    /// The time to seek in seconds
    #[structopt(long, value_name = "number")]
    pub seek_secs: Option<f32>,

    /// The amount of status updates per second
    #[structopt(long, value_name = "number")]
    pub ups: Option<f32>,
}

pub struct State {
    pub status: Status,
    pub queue: Vec<Track>,
    pub selected: usize,
    pub liststate: ListState,
    pub searching: bool,
    pub query: String,
    pub filtered: Vec<usize>,
}

#[derive(Debug)]
pub enum Command {
    Quit,
    UpdateFrame,
    UpdateStatus,
    UpdateQueue,
    ToggleRepeat,
    ToggleRandom,
    ToggleSingle,
    ToggleOneshot,
    ToggleConsume,
    TogglePause,
    Stop,
    SeekBackwards,
    SeekForwards,
    Previous,
    Next,
    Play,
    Reselect,
    Down,
    Up,
    JumpDown,
    JumpUp,
    InputSearch(char),
    BackspaceSearch,
    QuitSearch,
    Searching(bool),
}

impl State {
    pub fn update_search(&mut self, queue_strings: &[String]) {
        let query = self.query.to_lowercase();
        self.filtered.clear();
        for (i, track) in queue_strings.iter().enumerate() {
            if track.contains(&query) {
                self.filtered.push(i);
            }
        }
        self.selected = 0;
        self.liststate.select(None);
        self.liststate.select(Some(0));
    }
}
