#![allow(clippy::too_many_arguments)]
#![feature(box_patterns)]
#![forbid(unsafe_code)]

mod config;
mod defaults;
mod fail;
mod layout;
mod mpd;

use anyhow::{Context, Error, Result};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, MouseEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use dirs_next::config_dir;
use structopt::{clap::AppSettings, StructOpt};
use tokio::{
    sync::mpsc,
    time::{sleep_until, Duration, Instant},
};
use tui::{backend::CrosstermBackend, widgets::ListState, Terminal};

use std::{
    cmp::{max, min},
    fs,
    io::{stdout, Write},
    process::exit,
};

use crate::config::Config;

/// Minimal mpd terminal client that aims to be simple yet highly configurable
///
/// Homepage: https://github.com/figsoda/mmtc
#[derive(StructOpt)]
#[structopt(
    name = "mmtc",
    rename_all = "kebab-case",
    global_setting = AppSettings::ColoredHelp,
)]
struct Opts {
    /// Clear query on play
    #[structopt(long)]
    clear_query_on_play: bool,

    /// Cycle through the queue
    #[structopt(long)]
    cycle: bool,

    /// Don't clear query on play
    #[structopt(long, overrides_with("clear_query_on_play"))]
    no_clear_query_on_play: bool,

    /// Don't cycle through the queue
    #[structopt(long, overrides_with("cycle"))]
    no_cycle: bool,

    /// Specify the address of the mpd server
    #[structopt(long, value_name = "address")]
    address: Option<String>,

    /// Specify the config file
    #[structopt(short, long, value_name = "file")]
    config: Option<String>,

    /// The number of lines to jump
    #[structopt(long, value_name = "number")]
    jump_lines: Option<usize>,

    /// The time to seek in seconds
    #[structopt(long, value_name = "number")]
    seek_secs: Option<f64>,

    /// The amount of status updates per second
    #[structopt(long, value_name = "number")]
    ups: Option<f64>,
}

#[derive(Debug)]
enum Command {
    Quit,
    UpdateFrame,
    UpdateQueue,
    UpdateStatus,
    ToggleRepeat,
    ToggleRandom,
    ToggleSingle,
    ToggleConsume,
    ToggleOneshot,
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
    UpdateSearch,
    QuitSearch,
    Searching(bool),
}

fn cleanup() -> Result<()> {
    disable_raw_mode().context("Failed to disable raw mode")?;
    execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture)
        .context("Failed to clean up terminal")?;
    Ok(())
}

fn die<T>(e: impl Into<Error>) -> T {
    eprintln!("{:?}", cleanup().map_or_else(|x| x, |_| e.into()));
    exit(1);
}

#[tokio::main]
async fn main() {
    let res = run().await;
    if let Err(e) = cleanup().and(res) {
        eprintln!("{:?}", e);
        exit(1);
    }
    exit(0);
}

async fn run() -> Result<()> {
    let opts = Opts::from_args();

    let cfg: Config = if let Some(ref cfg_file) = opts.config {
        ron::de::from_bytes(&fs::read(cfg_file).with_context(fail::read(cfg_file))?)
            .with_context(fail::parse_cfg(cfg_file))?
    } else if let Some(xs) = config_dir() {
        let mut xs = xs;
        xs.push("mmtc");
        xs.push("mmtc.ron");

        if xs.is_file() {
            ron::de::from_bytes(&fs::read(&xs).with_context(fail::read(xs.display()))?)
                .with_context(fail::parse_cfg(xs.display()))?
        } else {
            defaults::config()
        }
    } else {
        defaults::config()
    };

    let addr = &if let Some(addr) = opts.address {
        addr.parse().with_context(fail::parse_addr(addr))?
    } else {
        cfg.address
    };
    let clear_query_on_play = opts.clear_query_on_play
        || if opts.no_clear_query_on_play {
            false
        } else {
            cfg.clear_query_on_play
        };
    let cycle = opts.cycle || if opts.no_cycle { false } else { cfg.cycle };
    let jump_lines = opts.jump_lines.unwrap_or(cfg.jump_lines);
    let seek_secs = opts.seek_secs.unwrap_or(cfg.seek_secs);

    let mut idle_cl = mpd::init(addr).await?;
    let cl = &mut mpd::init(addr).await?;

    let (mut queue, mut queue_strings) = mpd::queue(&mut idle_cl, &cfg.search_fields).await?;
    let mut status = mpd::status(cl).await?;
    let mut selected = status.song.map_or(0, |song| song.pos);
    let mut liststate = ListState::default();
    liststate.select(Some(selected));
    let mut searching = false;
    let mut query = String::with_capacity(32);
    let mut filtered = Vec::new();

    let seek_backwards = format!("seekcur -{}\n", seek_secs);
    let seek_backwards = seek_backwards.as_bytes();
    let seek_forwards = format!("seekcur +{}\n", seek_secs);
    let seek_forwards = seek_forwards.as_bytes();
    let update_interval = Duration::from_secs_f64(1.0 / opts.ups.unwrap_or(cfg.ups));

    let (tx, mut rx) = mpsc::channel(32);
    let tx1 = tx.clone();
    let tx2 = tx.clone();
    let tx3 = tx.clone();

    tokio::spawn(async move {
        let tx = tx1;
        loop {
            let changed = mpd::idle(&mut idle_cl)
                .await
                .context("Failed to idle")
                .unwrap_or_else(die);
            if changed.0 {
                tx.send(Command::UpdateQueue).await.unwrap_or_else(die);
            }
            if changed.1 {
                tx.send(Command::UpdateStatus).await.unwrap_or_else(die);
            }
            tx.send(Command::UpdateFrame).await.unwrap_or_else(die);
        }
    });

    tokio::spawn(async move {
        let tx = tx2;
        loop {
            let deadline = Instant::now() + update_interval;
            tx.send(Command::UpdateStatus).await.unwrap_or_else(die);
            tx.send(Command::UpdateFrame).await.unwrap_or_else(die);
            sleep_until(deadline).await;
        }
    });

    let mut stdout = stdout();
    enable_raw_mode().context("Failed to initialize terminal")?;
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .context("Failed to initialize terminal")?;
    let mut term =
        Terminal::new(CrosstermBackend::new(stdout)).context("Failed to initialize terminal")?;

    tokio::spawn(async move {
        let tx = tx3;
        while let Ok(ev) = event::read() {
            if let Some(cmd) = match ev {
                Event::Mouse(MouseEvent::ScrollDown(..)) => Some(Command::Down),
                Event::Mouse(MouseEvent::ScrollUp(..)) => Some(Command::Up),
                Event::Resize(..) => Some(Command::UpdateFrame),
                Event::Key(KeyEvent { code, .. }) => match code {
                    KeyCode::Esc => {
                        searching = false;
                        Some(Command::QuitSearch)
                    }
                    KeyCode::Down => Some(Command::Down),
                    KeyCode::Up => Some(Command::Up),
                    KeyCode::PageDown => Some(Command::JumpDown),
                    KeyCode::PageUp => Some(Command::JumpUp),
                    _ => {
                        if searching {
                            match code {
                                KeyCode::Char(c) => Some(Command::InputSearch(c)),
                                KeyCode::Backspace => Some(Command::BackspaceSearch),
                                KeyCode::Enter => {
                                    searching = false;
                                    Some(Command::Searching(false))
                                }
                                _ => None,
                            }
                        } else {
                            match code {
                                KeyCode::Char('q') => Some(Command::Quit),
                                KeyCode::Char('r') => Some(Command::ToggleRepeat),
                                KeyCode::Char('R') => Some(Command::ToggleRandom),
                                KeyCode::Char('s') => Some(Command::ToggleSingle),
                                KeyCode::Char('S') => Some(Command::ToggleOneshot),
                                KeyCode::Char('c') => Some(Command::ToggleConsume),
                                KeyCode::Char('p') => Some(Command::TogglePause),
                                KeyCode::Char(';') => Some(Command::Stop),
                                KeyCode::Char('h') | KeyCode::Left => Some(Command::SeekBackwards),
                                KeyCode::Char('l') | KeyCode::Right => Some(Command::SeekForwards),
                                KeyCode::Char('H') => Some(Command::Previous),
                                KeyCode::Char('L') => Some(Command::Next),
                                KeyCode::Enter => Some(Command::Play),
                                KeyCode::Char(' ') => Some(Command::Reselect),
                                KeyCode::Char('j') => Some(Command::Down),
                                KeyCode::Char('k') | KeyCode::Up => Some(Command::Up),
                                KeyCode::Char('J') | KeyCode::PageDown => Some(Command::JumpDown),
                                KeyCode::Char('K') | KeyCode::PageUp => Some(Command::JumpUp),
                                KeyCode::Char('/') => {
                                    searching = true;
                                    Some(Command::Searching(true))
                                }
                                _ => None,
                            }
                        }
                    }
                },
                _ => None,
            } {
                tx.send(cmd).await.unwrap_or_else(die);
            }
        }
    });

    while let Some(cmd) = rx.recv().await {
        match cmd {
            Command::Quit => break,
            Command::UpdateFrame => term
                .draw(|frame| {
                    layout::render(
                        frame,
                        frame.size(),
                        &cfg.layout,
                        &queue,
                        searching,
                        &query,
                        &filtered,
                        &status,
                        &mut liststate,
                    );
                })
                .context("Failed to draw to terminal")?,
            Command::UpdateQueue => {
                let res = mpd::queue(cl, &cfg.search_fields)
                    .await
                    .context("Failed to query queue")?;
                queue = res.0;
                queue_strings = res.1;
                selected = status.song.map_or(0, |song| song.pos);
                liststate = ListState::default();
                liststate.select(Some(selected));
                if !query.is_empty() {
                    tx.send(Command::UpdateSearch).await?;
                }
            }
            Command::UpdateStatus => {
                status = mpd::status(cl).await.context("Failed to query status")?;
            }
            Command::ToggleRepeat => {
                mpd::command(
                    cl,
                    if status.repeat {
                        b"repeat 0\n"
                    } else {
                        b"repeat 1\n"
                    },
                )
                .await
                .context("Failed to toggle repeat")?;
                tx.send(Command::UpdateStatus).await?;
                tx.send(Command::UpdateFrame).await?;
            }
            Command::ToggleRandom => {
                mpd::command(
                    cl,
                    if status.random {
                        b"random 0\n"
                    } else {
                        b"random 1\n"
                    },
                )
                .await
                .context("Failed to toggle random")?;
                tx.send(Command::UpdateStatus).await?;
                tx.send(Command::UpdateFrame).await?;
            }
            Command::ToggleSingle => {
                mpd::command(
                    cl,
                    if status.single == Some(true) {
                        b"single 0\n"
                    } else {
                        b"single 1\n"
                    },
                )
                .await
                .context("Failed to toggle single")?;
                tx.send(Command::UpdateStatus).await?;
                tx.send(Command::UpdateFrame).await?;
            }
            Command::ToggleOneshot => {
                mpd::command(
                    cl,
                    status.single.map_or(b"single 0\n", |_| b"single oneshot\n"),
                )
                .await
                .context("Failed to toggle oneshot")?;
                tx.send(Command::UpdateStatus).await?;
                tx.send(Command::UpdateFrame).await?;
            }
            Command::ToggleConsume => {
                mpd::command(
                    cl,
                    if status.consume {
                        b"consume 0\n"
                    } else {
                        b"consume 1\n"
                    },
                )
                .await
                .context("Failed to toggle consume")?;
                tx.send(Command::UpdateStatus).await?;
                tx.send(Command::UpdateFrame).await?;
            }
            Command::Stop => {
                mpd::command(cl, b"stop\n")
                    .await
                    .context("Faield to stop playing")?;
                tx.send(Command::UpdateStatus).await?;
                tx.send(Command::UpdateFrame).await?;
            }
            Command::SeekBackwards => {
                mpd::command(cl, seek_backwards)
                    .await
                    .context("Failed to seek backwards")?;
                tx.send(Command::UpdateStatus).await?;
                tx.send(Command::UpdateFrame).await?;
            }
            Command::SeekForwards => {
                mpd::command(cl, seek_forwards)
                    .await
                    .context("Failed to seek forwards")?;
                tx.send(Command::UpdateStatus).await?;
                tx.send(Command::UpdateFrame).await?;
            }
            Command::TogglePause => {
                mpd::command(cl, status.state.map_or(b"play\n", |_| b"pause\n"))
                    .await
                    .context("Failed to toggle pause")?;
                tx.send(Command::UpdateStatus).await?;
                tx.send(Command::UpdateFrame).await?;
            }
            Command::Previous => {
                mpd::command(cl, b"previous\n")
                    .await
                    .context("Failed to play previous song")?;
                tx.send(Command::UpdateStatus).await?;
                tx.send(Command::UpdateFrame).await?;
            }
            Command::Next => {
                mpd::command(cl, b"next\n")
                    .await
                    .context("Failed to play previous song")?;
                tx.send(Command::UpdateStatus).await?;
                tx.send(Command::UpdateFrame).await?;
            }
            Command::Play => {
                if query.is_empty() {
                    if selected < queue.len() {
                        mpd::play(cl, selected)
                            .await
                            .context("Failed to play the selected song")?;
                    }
                } else if selected < filtered.len() {
                    mpd::play(cl, filtered[selected])
                        .await
                        .context("Failed to play the selected song")?;
                };
                tx.send(Command::UpdateStatus).await?;
                if clear_query_on_play {
                    tx.send(Command::QuitSearch).await?;
                }
                tx.send(Command::UpdateFrame).await?;
            }
            Command::Reselect => {
                selected = status.song.map_or(0, |song| song.pos);
                liststate.select(Some(selected));
                tx.send(Command::UpdateFrame).await?;
            }
            Command::Down => {
                let len = if query.is_empty() {
                    queue.len()
                } else {
                    filtered.len()
                };
                if selected >= len {
                    selected = status.song.map_or(0, |song| song.pos);
                } else if selected == len - 1 {
                    if cycle {
                        selected = 0;
                    }
                } else {
                    selected += 1;
                }
                liststate.select(Some(selected));
                tx.send(Command::UpdateFrame).await?;
            }
            Command::Up => {
                let len = if query.is_empty() {
                    queue.len()
                } else {
                    filtered.len()
                };
                if selected >= len {
                    selected = status.song.map_or(0, |song| song.pos);
                } else if selected == 0 {
                    if cycle {
                        selected = len - 1;
                    }
                } else {
                    selected -= 1;
                }
                liststate.select(Some(selected));
                tx.send(Command::UpdateFrame).await?;
            }
            Command::JumpDown => {
                let len = if query.is_empty() {
                    queue.len()
                } else {
                    filtered.len()
                };
                selected = if selected >= len {
                    status.song.map_or(0, |song| song.pos)
                } else if cycle {
                    (selected + jump_lines) % len
                } else {
                    min(selected + jump_lines, len - 1)
                };
                liststate.select(Some(selected));
                tx.send(Command::UpdateFrame).await?;
            }
            Command::JumpUp => {
                let len = if query.is_empty() {
                    queue.len()
                } else {
                    filtered.len()
                };
                selected = if selected >= len {
                    status.song.map_or(0, |song| song.pos)
                } else if cycle {
                    ((selected as isize - jump_lines as isize) % len as isize) as usize
                } else {
                    max(selected as isize - jump_lines as isize, 0) as usize
                };
                liststate.select(Some(selected));
                tx.send(Command::UpdateFrame).await?;
            }
            Command::InputSearch(c) => {
                if query.is_empty() {
                    query.push(c);
                    tx.send(Command::UpdateSearch).await?;
                } else {
                    query.push(c);
                    let mut count = 0;
                    for i in filtered.clone() {
                        if queue_strings[i].contains(&query) {
                            filtered[count] = i;
                            count += 1;
                        }
                    }
                    filtered.truncate(count);
                    tx.send(Command::UpdateFrame).await?;
                }
            }
            Command::BackspaceSearch => {
                let c = query.pop();
                if !query.is_empty() {
                    tx.send(Command::UpdateSearch).await?;
                } else if c.is_some() {
                    selected = status.song.map_or(0, |song| song.pos);
                    liststate.select(Some(selected));
                    tx.send(Command::UpdateFrame).await?;
                }
            }
            Command::UpdateSearch => {
                let query = query.to_lowercase();
                filtered.clear();
                for (i, track) in queue_strings.iter().enumerate() {
                    if track.contains(&query) {
                        filtered.push(i);
                    }
                }
                selected = 0;
                liststate.select(None);
                liststate.select(Some(0));
                tx.send(Command::UpdateFrame).await?;
            }
            Command::QuitSearch => {
                searching = false;
                if !query.is_empty() {
                    query.clear();
                    selected = status.song.map_or(0, |song| song.pos);
                    liststate.select(Some(selected));
                }
                tx.send(Command::UpdateFrame).await?;
            }
            Command::Searching(x) => {
                searching = x;
                tx.send(Command::UpdateFrame).await?;
            }
        }
    }

    Ok(())
}
