#![allow(clippy::too_many_arguments)]
#![feature(box_patterns)]
#![forbid(unsafe_code)]

mod app;
mod config;
mod defaults;
mod fail;
mod layout;
mod mpd;

use anyhow::{Context, Error, Result};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
        MouseEvent,
    },
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use dirs_next::config_dir;
use structopt::StructOpt;
use tokio::{
    sync::mpsc,
    time::{sleep_until, Duration, Instant},
};
use tui::{backend::CrosstermBackend, widgets::ListState, Terminal};

use std::{cmp::min, fs, io::stdout, process::exit};

use crate::{
    app::{Command, Opts, State},
    config::Config,
    layout::render,
    mpd::Client,
};

fn cleanup() -> Result<()> {
    let mut stdout = stdout();
    stdout
        .execute(LeaveAlternateScreen)
        .context("Failed to leave alternate screen")?;
    stdout
        .execute(DisableMouseCapture)
        .context("Failed to disable mouse capture")?;
    disable_raw_mode().context("Failed to disable raw mode")?;
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

    let cfg: Config = if let Some(file) = opts.config {
        ron::de::from_bytes(&fs::read(&file).with_context(fail::read(file.display()))?)
            .with_context(fail::parse_cfg(file.display()))?
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

    let addr = &opts.address.unwrap_or(cfg.address);
    let mut idle_cl = Client::init(addr).await?;
    let mut cl = Client::init(addr).await?;

    let status = cl.status().await?;
    let (queue, mut queue_strings) = idle_cl.queue(status.queue_len, &cfg.search_fields).await?;
    let mut s = State {
        selected: status.song.map_or(0, |song| song.pos),
        status,
        queue,
        liststate: ListState::default(),
        searching: false,
        query: String::with_capacity(32),
        filtered: Vec::new(),
    };
    s.liststate.select(Some(s.selected));

    enable_raw_mode().context("Failed to enable raw mode")?;
    let mut stdout = stdout();
    stdout
        .execute(EnableMouseCapture)
        .context("Failed to enable mouse capture")?;
    stdout
        .execute(EnterAlternateScreen)
        .context("Failed to enter alternate screen")?;
    let mut term =
        Terminal::new(CrosstermBackend::new(stdout)).context("Failed to initialize terminal")?;

    render(&mut term, &cfg.layout, &mut s)?;

    let clear_query_on_play = opts.clear_query_on_play
        || if opts.no_clear_query_on_play {
            false
        } else {
            cfg.clear_query_on_play
        };
    let cycle = opts.cycle || if opts.no_cycle { false } else { cfg.cycle };
    let jump_lines = opts.jump_lines.unwrap_or(cfg.jump_lines);
    let seek_secs = opts.seek_secs.unwrap_or(cfg.seek_secs);

    let seek_backwards = format!("seekcur -{}\n", seek_secs);
    let seek_backwards = seek_backwards.as_bytes();
    let seek_forwards = format!("seekcur +{}\n", seek_secs);
    let seek_forwards = seek_forwards.as_bytes();
    let update_interval = Duration::from_secs_f32(1.0 / opts.ups.unwrap_or(cfg.ups));

    let (tx, mut rx) = mpsc::channel(32);
    let tx1 = tx.clone();
    let tx2 = tx.clone();
    let tx3 = tx.clone();

    tokio::spawn(async move {
        let tx = tx1;
        loop {
            let changed = idle_cl.idle().await.unwrap_or_else(die);
            if changed.0 {
                tx.send(Command::UpdateStatus).await.unwrap_or_else(die);
            }
            if changed.1 {
                tx.send(Command::UpdateQueue).await.unwrap_or_else(die);
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

    tokio::spawn(async move {
        let mut searching = false;
        let tx = tx3;
        while let Ok(ev) = event::read() {
            tx.send(match ev {
                Event::Mouse(MouseEvent::ScrollDown(..)) => Command::Down,
                Event::Mouse(MouseEvent::ScrollUp(..)) => Command::Up,
                Event::Resize(..) => Command::UpdateFrame,
                Event::Key(KeyEvent { code, modifiers }) => match code {
                    KeyCode::Char('q') if modifiers.contains(KeyModifiers::CONTROL) => {
                        Command::Quit
                    }
                    KeyCode::Esc => {
                        searching = false;
                        Command::QuitSearch
                    }
                    KeyCode::Left => Command::SeekBackwards,
                    KeyCode::Right => Command::SeekForwards,
                    KeyCode::Down => Command::Down,
                    KeyCode::Up => Command::Up,
                    KeyCode::PageDown => Command::JumpDown,
                    KeyCode::PageUp => Command::JumpUp,
                    KeyCode::Enter if searching => {
                        searching = false;
                        Command::Searching(false)
                    }
                    KeyCode::Enter => Command::Play,
                    KeyCode::Backspace if searching => Command::BackspaceSearch,
                    KeyCode::Char(c) if searching => Command::InputSearch(c),
                    KeyCode::Char(c) => match c {
                        'q' => Command::Quit,
                        'r' => Command::ToggleRepeat,
                        'R' => Command::ToggleRandom,
                        's' => Command::ToggleSingle,
                        'S' => Command::ToggleOneshot,
                        'c' => Command::ToggleConsume,
                        'p' => Command::TogglePause,
                        ';' => Command::Stop,
                        'h' => Command::SeekBackwards,
                        'l' => Command::SeekForwards,
                        'H' => Command::Previous,
                        'L' => Command::Next,
                        ' ' => Command::Reselect,
                        'j' => Command::Down,
                        'k' => Command::Up,
                        'J' => Command::JumpDown,
                        'K' => Command::JumpUp,
                        '/' => {
                            searching = true;
                            Command::Searching(true)
                        }
                        _ => continue,
                    },
                    _ => continue,
                },
                _ => continue,
            })
            .await
            .unwrap_or_else(die);
        }
    });

    while let Some(cmd) = rx.recv().await {
        match cmd {
            Command::Quit => break,
            Command::UpdateFrame => render(&mut term, &cfg.layout, &mut s)?,
            Command::UpdateQueue => {
                let res = cl.queue(s.status.queue_len, &cfg.search_fields).await?;
                s.queue = res.0;
                queue_strings = res.1;
                s.selected = s.status.song.map_or(0, |song| song.pos);
                s.liststate = ListState::default();
                s.liststate.select(Some(s.selected));
                if !s.query.is_empty() {
                    s.update_search(&queue_strings);
                }
            }
            Command::UpdateStatus => {
                s.status = cl.status().await?;
            }
            Command::ToggleRepeat => {
                cl.command(if s.status.repeat {
                    b"repeat 0\n"
                } else {
                    b"repeat 1\n"
                })
                .await
                .context("Failed to toggle repeat")?;
                s.status = cl.status().await?;
                render(&mut term, &cfg.layout, &mut s)?;
            }
            Command::ToggleRandom => {
                cl.command(if s.status.random {
                    b"random 0\n"
                } else {
                    b"random 1\n"
                })
                .await
                .context("Failed to toggle random")?;
                s.status = cl.status().await?;
                render(&mut term, &cfg.layout, &mut s)?;
            }
            Command::ToggleSingle => {
                cl.command(if s.status.single == Some(true) {
                    b"single 0\n"
                } else {
                    b"single 1\n"
                })
                .await
                .context("Failed to toggle single")?;
                s.status = cl.status().await?;
                render(&mut term, &cfg.layout, &mut s)?;
            }
            Command::ToggleOneshot => {
                cl.command(
                    s.status
                        .single
                        .map_or(b"single 0\n", |_| b"single oneshot\n"),
                )
                .await
                .context("Failed to toggle oneshot")?;
                s.status = cl.status().await?;
                render(&mut term, &cfg.layout, &mut s)?;
            }
            Command::ToggleConsume => {
                cl.command(if s.status.consume {
                    b"consume 0\n"
                } else {
                    b"consume 1\n"
                })
                .await
                .context("Failed to toggle consume")?;
                s.status = cl.status().await?;
                render(&mut term, &cfg.layout, &mut s)?;
            }
            Command::Stop => {
                cl.command(b"stop\n")
                    .await
                    .context("Failed to stop playing")?;
                s.status = cl.status().await?;
                render(&mut term, &cfg.layout, &mut s)?;
            }
            Command::SeekBackwards => {
                cl.command(seek_backwards)
                    .await
                    .context("Failed to seek backwards")?;
                s.status = cl.status().await?;
                render(&mut term, &cfg.layout, &mut s)?;
            }
            Command::SeekForwards => {
                cl.command(seek_forwards)
                    .await
                    .context("Failed to seek forwards")?;
                s.status = cl.status().await?;
                render(&mut term, &cfg.layout, &mut s)?;
            }
            Command::TogglePause => {
                cl.command(s.status.state.map_or(b"play\n", |_| b"pause\n"))
                    .await
                    .context("Failed to toggle pause")?;
                s.status = cl.status().await?;
                render(&mut term, &cfg.layout, &mut s)?;
            }
            Command::Previous => {
                cl.command(b"previous\n")
                    .await
                    .context("Failed to play previous song")?;
                s.status = cl.status().await?;
                render(&mut term, &cfg.layout, &mut s)?;
            }
            Command::Next => {
                cl.command(b"next\n")
                    .await
                    .context("Failed to play next song")?;
                s.status = cl.status().await?;
                render(&mut term, &cfg.layout, &mut s)?;
            }
            Command::Play => {
                cl.play(if s.query.is_empty() {
                    if s.selected < s.queue.len() {
                        s.selected
                    } else {
                        continue;
                    }
                } else if let Some(&x) = s.filtered.get(s.selected) {
                    x
                } else {
                    continue;
                })
                .await
                .context("Failed to play the selected song")?;
                s.status = cl.status().await?;
                if clear_query_on_play {
                    tx.send(Command::QuitSearch).await?;
                } else {
                    render(&mut term, &cfg.layout, &mut s)?;
                }
            }
            Command::Reselect => {
                s.selected = s.status.song.map_or(0, |song| song.pos);
                s.liststate.select(Some(s.selected));
                render(&mut term, &cfg.layout, &mut s)?;
            }
            Command::Down => {
                let len = if s.query.is_empty() {
                    s.queue.len()
                } else {
                    s.filtered.len()
                };
                if s.selected >= len {
                    s.selected = s.status.song.map_or(0, |song| song.pos);
                } else if s.selected == len - 1 {
                    if cycle {
                        s.selected = 0;
                    }
                } else {
                    s.selected += 1;
                }
                s.liststate.select(Some(s.selected));
                render(&mut term, &cfg.layout, &mut s)?;
            }
            Command::Up => {
                let len = if s.query.is_empty() {
                    s.queue.len()
                } else {
                    s.filtered.len()
                };
                if s.selected >= len {
                    s.selected = s.status.song.map_or(0, |song| song.pos);
                } else if s.selected == 0 {
                    if cycle {
                        s.selected = len - 1;
                    }
                } else {
                    s.selected -= 1;
                }
                s.liststate.select(Some(s.selected));
                render(&mut term, &cfg.layout, &mut s)?;
            }
            Command::JumpDown => {
                let len = if s.query.is_empty() {
                    s.queue.len()
                } else {
                    s.filtered.len()
                };
                s.selected = if s.selected >= len {
                    s.status.song.map_or(0, |song| song.pos)
                } else if cycle {
                    (s.selected + jump_lines) % len
                } else {
                    min(s.selected + jump_lines, len - 1)
                };
                s.liststate.select(Some(s.selected));
                render(&mut term, &cfg.layout, &mut s)?;
            }
            Command::JumpUp => {
                let len = if s.query.is_empty() {
                    s.queue.len()
                } else {
                    s.filtered.len()
                };
                s.selected = if s.selected >= len {
                    s.status.song.map_or(0, |song| song.pos)
                } else if cycle {
                    ((s.selected as isize - jump_lines as isize) % len as isize) as usize
                } else if s.selected < jump_lines {
                    0
                } else {
                    s.selected - jump_lines
                };
                s.liststate.select(Some(s.selected));
                render(&mut term, &cfg.layout, &mut s)?;
            }
            Command::InputSearch(c) => {
                if s.query.is_empty() {
                    s.query.push(c);
                    s.update_search(&queue_strings);
                } else {
                    s.query.push(c);
                    let query = s.query.to_lowercase();
                    s.filtered.retain(|&i| queue_strings[i].contains(&query));
                }
                render(&mut term, &cfg.layout, &mut s)?;
            }
            Command::BackspaceSearch => {
                let c = s.query.pop();
                if !s.query.is_empty() {
                    s.update_search(&queue_strings);
                } else if c.is_some() {
                    s.selected = s.status.song.map_or(0, |song| song.pos);
                    s.liststate.select(Some(s.selected));
                }
                render(&mut term, &cfg.layout, &mut s)?;
            }
            Command::QuitSearch => {
                s.searching = false;
                if !s.query.is_empty() {
                    s.query.clear();
                    s.selected = s.status.song.map_or(0, |song| song.pos);
                    s.liststate.select(Some(s.selected));
                }
                render(&mut term, &cfg.layout, &mut s)?;
            }
            Command::Searching(x) => {
                s.searching = x;
                render(&mut term, &cfg.layout, &mut s)?;
            }
        }
    }

    Ok(())
}
