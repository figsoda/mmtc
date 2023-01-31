use secular::lower_lay_string;
use tui::widgets::ListState;

use crate::mpd::{Status, Track};

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
    GotoTop,
    GotoBottom,
    InputSearch(char),
    BackspaceSearch,
    ClearSearch,
    QuitSearch,
    Searching(bool),
}

impl State {
    pub fn select(&mut self, x: usize) {
        self.selected = x;
        self.liststate.select(Some(x));
    }

    pub fn reselect(&mut self) {
        self.select(self.status.song.as_ref().map_or(0, |song| song.pos));
    }

    pub fn len(&self) -> usize {
        if self.query.is_empty() {
            self.queue.len()
        } else {
            self.filtered.len()
        }
    }

    pub fn update_search(&mut self, queue_strings: &[String]) {
        let query = lower_lay_string(&self.query);
        self.filtered.clear();
        for (i, track) in queue_strings.iter().enumerate() {
            if track.contains(&query) {
                self.filtered.push(i);
            }
        }
        self.liststate.select(None);
        self.select(0);
    }

    pub fn quit_search(&mut self) {
        self.searching = false;
        if !self.query.is_empty() {
            self.query.clear();
            self.reselect();
        }
    }
}
