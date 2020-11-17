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
    app::{Command, Opts},
    config::Config,
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

    let mut status = cl.status().await?;
    let (mut queue, mut queue_strings) =
        idle_cl.queue(status.queue_len, &cfg.search_fields).await?;
    let mut selected = status.song.map_or(0, |song| song.pos);
    let mut liststate = ListState::default();
    liststate.select(Some(selected));
    let mut searching = false;
    let mut query = String::with_capacity(32);
    let mut filtered = Vec::new();

    macro_rules! update_search {
        () => {{
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
        }};
    }

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

    macro_rules! render {
        () => {
            term.draw(|frame| {
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
            .context("Failed to draw to terminal")?
        };
    };
    render!();

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
            Command::UpdateFrame => render!(),
            Command::UpdateQueue => {
                let res = cl.queue(status.queue_len, &cfg.search_fields).await?;
                queue = res.0;
                queue_strings = res.1;
                selected = status.song.map_or(0, |song| song.pos);
                liststate = ListState::default();
                liststate.select(Some(selected));
                if !query.is_empty() {
                    update_search!();
                }
            }
            Command::UpdateStatus => {
                status = cl.status().await?;
            }
            Command::ToggleRepeat => {
                cl.command(if status.repeat {
                    b"repeat 0\n"
                } else {
                    b"repeat 1\n"
                })
                .await
                .context("Failed to toggle repeat")?;
                status = cl.status().await?;
                render!();
            }
            Command::ToggleRandom => {
                cl.command(if status.random {
                    b"random 0\n"
                } else {
                    b"random 1\n"
                })
                .await
                .context("Failed to toggle random")?;
                status = cl.status().await?;
                render!();
            }
            Command::ToggleSingle => {
                cl.command(if status.single == Some(true) {
                    b"single 0\n"
                } else {
                    b"single 1\n"
                })
                .await
                .context("Failed to toggle single")?;
                status = cl.status().await?;
                render!();
            }
            Command::ToggleOneshot => {
                cl.command(status.single.map_or(b"single 0\n", |_| b"single oneshot\n"))
                    .await
                    .context("Failed to toggle oneshot")?;
                status = cl.status().await?;
                render!();
            }
            Command::ToggleConsume => {
                cl.command(if status.consume {
                    b"consume 0\n"
                } else {
                    b"consume 1\n"
                })
                .await
                .context("Failed to toggle consume")?;
                status = cl.status().await?;
                render!();
            }
            Command::Stop => {
                cl.command(b"stop\n")
                    .await
                    .context("Failed to stop playing")?;
                status = cl.status().await?;
                render!();
            }
            Command::SeekBackwards => {
                cl.command(seek_backwards)
                    .await
                    .context("Failed to seek backwards")?;
                status = cl.status().await?;
                render!();
            }
            Command::SeekForwards => {
                cl.command(seek_forwards)
                    .await
                    .context("Failed to seek forwards")?;
                status = cl.status().await?;
                render!();
            }
            Command::TogglePause => {
                cl.command(status.state.map_or(b"play\n", |_| b"pause\n"))
                    .await
                    .context("Failed to toggle pause")?;
                status = cl.status().await?;
                render!();
            }
            Command::Previous => {
                cl.command(b"previous\n")
                    .await
                    .context("Failed to play previous song")?;
                status = cl.status().await?;
                render!();
            }
            Command::Next => {
                cl.command(b"next\n")
                    .await
                    .context("Failed to play next song")?;
                status = cl.status().await?;
                render!();
            }
            Command::Play => {
                cl.play(if query.is_empty() {
                    if selected < queue.len() {
                        selected
                    } else {
                        continue;
                    }
                } else if let Some(&x) = filtered.get(selected) {
                    x
                } else {
                    continue;
                })
                .await
                .context("Failed to play the selected song")?;
                status = cl.status().await?;
                if clear_query_on_play {
                    tx.send(Command::QuitSearch).await?;
                } else {
                    render!();
                }
            }
            Command::Reselect => {
                selected = status.song.map_or(0, |song| song.pos);
                liststate.select(Some(selected));
                render!();
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
                render!();
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
                render!();
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
                render!();
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
                } else if selected < jump_lines {
                    0
                } else {
                    selected - jump_lines
                };
                liststate.select(Some(selected));
                render!();
            }
            Command::InputSearch(c) => {
                if query.is_empty() {
                    query.push(c);
                    update_search!();
                } else {
                    query.push(c);
                    filtered.retain(|&i| queue_strings[i].contains(&query));
                }
                render!();
            }
            Command::BackspaceSearch => {
                let c = query.pop();
                if !query.is_empty() {
                    update_search!();
                } else if c.is_some() {
                    selected = status.song.map_or(0, |song| song.pos);
                    liststate.select(Some(selected));
                }
                render!();
            }
            Command::QuitSearch => {
                searching = false;
                if !query.is_empty() {
                    query.clear();
                    selected = status.song.map_or(0, |song| song.pos);
                    liststate.select(Some(selected));
                }
                render!();
            }
            Command::Searching(x) => {
                searching = x;
                render!();
            }
        }
    }

    Ok(())
}
