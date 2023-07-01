#![forbid(unsafe_code)]

mod app;
mod cli;
mod config;
mod defaults;
mod layout;
mod mpd;

use std::{
    cmp::min,
    env, fs,
    io::stdout,
    process::exit,
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc,
    },
    thread::{self, Thread},
    time::Duration,
};

use anyhow::{Context, Result};
use async_io::{block_on, Timer};
use async_net::resolve;
use clap::Parser;
use crossbeam_queue::SegQueue;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
        MouseEvent, MouseEventKind,
    },
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use dirs::config_dir;
use futures_lite::StreamExt;
use ratatui::{backend::CrosstermBackend, widgets::ListState, Terminal};
use secular::lower_lay_string;

use crate::{
    app::{Command, State},
    cli::Opts,
    layout::render,
    mpd::{Client, PlayerState},
};

fn cleanup() {
    let mut stdout = stdout();

    if let Err(e) = stdout
        .execute(LeaveAlternateScreen)
        .context("Failed to leave alternate screen")
    {
        eprintln!("{e:?}");
    }

    if let Err(e) = stdout
        .execute(DisableMouseCapture)
        .context("Failed to disable mouse capture")
    {
        eprintln!("{e:?}");
    }

    if let Err(e) = disable_raw_mode().context("Failed to disable raw mode") {
        eprintln!("{e:?}");
    }
}

struct Cleanup;
impl Drop for Cleanup {
    fn drop(&mut self) {
        cleanup();
    }
}

fn main() -> Result<()> {
    block_on(run())
}

async fn run() -> Result<()> {
    let opts = Opts::parse();

    let cfg = if let Some(file) = opts.config {
        ron::de::from_bytes(
            &fs::read(&file).with_context(|| format!("Failed to read file {}", file.display()))?,
        )
        .with_context(|| format!("Failed to parse configuration file {}", file.display()))?
    } else if let Some(xs) = config_dir() {
        let xs = xs.join("mmtc").join("mmtc.ron");

        if xs.is_file() {
            ron::de::from_bytes(
                &fs::read(&xs).with_context(|| format!("Failed to read file {}", xs.display()))?,
            )
            .with_context(|| format!("Failed to parse configuration file {}", xs.display()))?
        } else {
            defaults::config()
        }
    } else {
        defaults::config()
    };

    let addr = &*if let Some(addr) = opts.address {
        resolve(addr).await?
    } else if let (Ok(host), Ok(port)) = (env::var("MPD_HOST"), env::var("MPD_PORT")) {
        resolve((host, port.parse()?)).await?
    } else {
        resolve(cfg.address).await?
    };

    let mut cl = Client::init(addr).await?;
    if let Some(cmd) = opts.cmd {
        for cmd in cmd {
            cl.command_stdout(&cmd).await?;
        }
        return Ok(());
    }

    let status = cl.status().await?;
    let mut idle_cl = Client::init(addr).await?;
    let (queue, mut queue_strings) = idle_cl.queue(status.queue_len, &cfg.search_fields).await?;
    let mut s = State {
        selected: 0,
        status,
        queue,
        liststate: ListState::default(),
        searching: false,
        query: String::with_capacity(32),
        filtered: Vec::new(),
    };
    s.reselect();

    enable_raw_mode().context("Failed to enable raw mode")?;
    let mut stdout = stdout();
    stdout
        .execute(EnableMouseCapture)
        .context("Failed to enable mouse capture")?;
    stdout
        .execute(EnterAlternateScreen)
        .context("Failed to enter alternate screen")?;
    let _cleanup = Cleanup;
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

    let seek_backwards = format!("seekcur -{seek_secs}");
    let seek_backwards = seek_backwards.as_bytes();
    let seek_forwards = format!("seekcur +{seek_secs}");
    let seek_forwards = seek_forwards.as_bytes();
    let update_interval = Duration::from_secs_f32(1.0 / opts.ups.unwrap_or(cfg.ups));

    let t1 = thread::current();
    let t2 = Thread::clone(&t1);
    let t3 = Thread::clone(&t1);
    // update status: 0b100
    // update queue:  0b010
    // update frame:  0b001
    let updates = Arc::new(AtomicU8::new(0b000));
    let updates1 = Arc::clone(&updates);
    let updates2 = Arc::clone(&updates);
    let updates3 = Arc::clone(&updates);
    let cmds = Arc::new(SegQueue::new());
    let cmds1 = Arc::clone(&cmds);

    thread::spawn(move || {
        block_on(async move {
            loop {
                updates1.fetch_or(
                    match idle_cl.idle().await {
                        Ok((true, true)) => 0b111,
                        Ok((true, false)) => 0b101,
                        Ok((false, true)) => 0b011,
                        Ok(_) => continue,
                        Err(e) => {
                            eprintln!("{e:?}");
                            exit(1);
                        }
                    },
                    Ordering::Relaxed,
                );
                t1.unpark();
            }
        })
    });

    thread::spawn(move || {
        block_on(async move {
            let mut timer = Timer::interval(update_interval);
            loop {
                updates2.fetch_or(0b101, Ordering::Relaxed);
                t2.unpark();
                timer.next().await;
            }
        })
    });

    thread::spawn(move || {
        let mut searching = false;
        while let Ok(ev) = event::read() {
            cmds1.push(match ev {
                Event::Mouse(MouseEvent {
                    kind: MouseEventKind::ScrollUp,
                    ..
                }) => Command::Up,
                Event::Mouse(MouseEvent {
                    kind: MouseEventKind::ScrollDown,
                    ..
                }) => Command::Down,
                Event::Resize(..) => {
                    updates3.fetch_or(0b001, Ordering::Relaxed);
                    t3.unpark();
                    continue;
                }
                Event::Key(KeyEvent {
                    code, modifiers, ..
                }) => match code {
                    KeyCode::Char('q') if modifiers.contains(KeyModifiers::CONTROL) => {
                        Command::Quit
                    }
                    KeyCode::Char('u') if modifiers.contains(KeyModifiers::CONTROL) => {
                        if searching {
                            Command::ClearSearch
                        } else {
                            Command::JumpUp
                        }
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
                    KeyCode::Esc => {
                        searching = false;
                        Command::QuitSearch
                    }
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
                        'd' if modifiers.contains(KeyModifiers::CONTROL) => Command::JumpDown,
                        'K' => Command::JumpUp,
                        'g' => Command::GotoTop,
                        'G' => Command::GotoBottom,
                        '/' => {
                            searching = true;
                            Command::Searching(true)
                        }
                        _ => continue,
                    },
                    _ => continue,
                },
                _ => continue,
            });
            t3.unpark();
        }
    });

    loop {
        let updates = if let Some(cmd) = cmds.pop() {
            (match cmd {
                Command::Quit => return Ok(()),
                Command::ToggleRepeat => {
                    cl.command(if s.status.repeat {
                        b"repeat 0"
                    } else {
                        b"repeat 1"
                    })
                    .await
                    .context("Failed to toggle repeat")?;
                    0b101
                }
                Command::ToggleRandom => {
                    cl.command(if s.status.random {
                        b"random 0"
                    } else {
                        b"random 1"
                    })
                    .await
                    .context("Failed to toggle random")?;
                    0b101
                }
                Command::ToggleSingle => {
                    cl.command(if s.status.single == Some(true) {
                        b"single 0"
                    } else {
                        b"single 1"
                    })
                    .await
                    .context("Failed to toggle single")?;
                    0b101
                }
                Command::ToggleOneshot => {
                    cl.command(s.status.single.map_or(b"single 0", |_| b"single oneshot"))
                        .await
                        .context("Failed to toggle oneshot")?;
                    0b101
                }
                Command::ToggleConsume => {
                    cl.command(if s.status.consume {
                        b"consume 0"
                    } else {
                        b"consume 1"
                    })
                    .await
                    .context("Failed to toggle consume")?;
                    0b101
                }
                Command::TogglePause => {
                    cl.command(match s.status.state {
                        PlayerState::Play => b"pause",
                        PlayerState::Pause => b"play",
                        _ => continue,
                    })
                    .await
                    .context("Failed to toggle pause")?;
                    0b101
                }
                Command::Stop => {
                    cl.command(b"stop")
                        .await
                        .context("Failed to stop playing")?;
                    0b101
                }
                Command::SeekBackwards => {
                    cl.command(seek_backwards)
                        .await
                        .context("Failed to seek backwards")?;
                    0b101
                }
                Command::SeekForwards => {
                    cl.command(seek_forwards)
                        .await
                        .context("Failed to seek forwards")?;
                    0b101
                }
                Command::Previous => {
                    cl.command(b"previous")
                        .await
                        .context("Failed to play previous song")?;
                    0b101
                }
                Command::Next => {
                    cl.command(b"next")
                        .await
                        .context("Failed to play next song")?;
                    0b101
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
                    if clear_query_on_play {
                        s.quit_search();
                    }
                    0b101
                }
                Command::Reselect => {
                    s.reselect();
                    0b001
                }
                Command::Down => {
                    let len = s.len();
                    if s.selected >= len {
                        s.reselect();
                    } else if s.selected == len - 1 {
                        if cycle {
                            s.select(0);
                        }
                    } else {
                        s.select(s.selected + 1);
                    }
                    0b001
                }
                Command::Up => {
                    let len = s.len();
                    if s.selected >= len {
                        s.reselect();
                    } else if s.selected == 0 {
                        if cycle {
                            s.select(len - 1);
                        }
                    } else {
                        s.select(s.selected - 1);
                    }
                    0b001
                }
                Command::JumpDown => {
                    let len = s.len();
                    if s.selected >= len {
                        s.reselect();
                    } else if cycle {
                        s.select((s.selected + jump_lines) % len);
                    } else {
                        s.select(min(s.selected + jump_lines, len - 1));
                    };
                    0b001
                }
                Command::JumpUp => {
                    let len = s.len();
                    if s.selected >= len {
                        s.reselect();
                    } else if cycle {
                        while s.selected < jump_lines {
                            s.selected += len;
                        }
                        s.selected -= jump_lines;
                        s.liststate.select(Some(s.selected));
                    } else if s.selected < jump_lines {
                        s.select(0);
                    } else {
                        s.select(s.selected - jump_lines);
                    };
                    0b001
                }
                Command::GotoTop => {
                    s.select(0);
                    0b001
                }
                Command::GotoBottom => {
                    let len = s.len();
                    if len == 0 {
                        continue;
                    }
                    s.select(len - 1);
                    0b001
                }
                Command::InputSearch(c) => {
                    let empty = s.query.is_empty();
                    s.query.push(c);
                    if empty {
                        s.update_search(&queue_strings);
                    } else {
                        let query = lower_lay_string(&s.query);
                        s.filtered.retain(|&i| queue_strings[i].contains(&query));
                    }
                    0b001
                }
                Command::BackspaceSearch => {
                    let c = s.query.pop();
                    if !s.query.is_empty() {
                        s.update_search(&queue_strings);
                    } else if c.is_some() {
                        s.reselect();
                    }
                    0b001
                }
                Command::ClearSearch => {
                    if !s.query.is_empty() {
                        s.query.clear();
                        s.reselect();
                    }
                    0b001
                }
                Command::QuitSearch => {
                    s.quit_search();
                    0b001
                }
                Command::Searching(x) => {
                    s.searching = x;
                    0b001
                }
            }) | updates.swap(0b000, Ordering::SeqCst)
        } else {
            match updates.swap(0b000, Ordering::SeqCst) {
                // wait for more commands or updates if neither were received
                x if x == 0b000 => {
                    thread::park();
                    continue;
                }
                x => x,
            }
        };

        // conditionally update status
        if updates & 0b100 == 0b100 {
            s.status = cl.status().await?;
        }

        // conditionally update queue
        if updates & 0b010 == 0b010 {
            (s.queue, queue_strings) = cl.queue(s.status.queue_len, &cfg.search_fields).await?;
            s.liststate.select(None);
            s.reselect();
            if !s.query.is_empty() {
                s.update_search(&queue_strings);
            }
        }

        // conditionally update frame
        if updates & 0b001 == 0b001 {
            render(&mut term, &cfg.layout, &mut s)?;
        }
    }
}
