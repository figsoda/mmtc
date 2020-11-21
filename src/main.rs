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
use async_io::{block_on, Timer};
use crossbeam_queue::SegQueue;
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
use tui::{backend::CrosstermBackend, widgets::ListState, Terminal};

use std::{
    cmp::min,
    fs,
    io::stdout,
    process::exit,
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use crate::{
    app::{Command, Opts, State},
    layout::render,
    mpd::Client,
};

trait OrDie<T> {
    fn or_die(self) -> T;
}

impl<T, E: Into<Error>> OrDie<T> for Result<T, E> {
    fn or_die(self) -> T {
        self.unwrap_or_else(|e| {
            eprintln!("{:?}", cleanup().map_or_else(|x| x, |_| e.into()));
            exit(1);
        })
    }
}

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

fn main() {
    let res = block_on(run());
    if let Err(e) = cleanup().and(res) {
        eprintln!("{:?}", e);
        exit(1);
    }
}

async fn run() -> Result<()> {
    let opts = Opts::from_args();

    let cfg = if let Some(file) = opts.config {
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

    let t1 = thread::current();
    let t2 = t1.clone();
    let t3 = t1.clone();
    // update status: 0b100
    // update queue:  0b010
    // update frame:  0b001
    let updates = Arc::new(AtomicU8::new(0b000));
    let updates1 = updates.clone();
    let updates2 = updates.clone();
    let updates3 = updates.clone();
    let cmds = Arc::new(SegQueue::new());
    let cmds1 = cmds.clone();

    thread::spawn(move || {
        block_on(async move {
            loop {
                updates1.fetch_or(
                    match idle_cl.idle().await.or_die() {
                        (true, true) => 0b111,
                        (true, false) => 0b101,
                        (false, true) => 0b011,
                        _ => continue,
                    },
                    Ordering::Relaxed,
                );
                t1.unpark();
            }
        })
    });

    thread::spawn(move || {
        block_on(async move {
            loop {
                let timer = Timer::after(update_interval);
                updates2.fetch_or(0b101, Ordering::Relaxed);
                t2.unpark();
                timer.await;
            }
        })
    });

    thread::spawn(move || {
        let mut searching = false;
        while let Ok(ev) = event::read() {
            cmds1.push(match ev {
                Event::Mouse(MouseEvent::ScrollDown(..)) => Command::Down,
                Event::Mouse(MouseEvent::ScrollUp(..)) => Command::Up,
                Event::Resize(..) => {
                    updates3.fetch_or(0b001, Ordering::Relaxed);
                    t3.unpark();
                    continue;
                }
                Event::Key(KeyEvent { code, modifiers }) => match code {
                    KeyCode::Char('q') if modifiers.contains(KeyModifiers::CONTROL) => {
                        Command::Quit
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
            });
            t3.unpark();
        }
    });

    loop {
        let mut empty = false;
        let cmd = cmds.pop();
        let mut updates = updates.swap(0b000, Ordering::SeqCst);
        match cmd {
            Some(cmd) => match cmd {
                Command::Quit => return Ok(()),
                Command::ToggleRepeat => {
                    cl.command(if s.status.repeat {
                        b"repeat 0\n"
                    } else {
                        b"repeat 1\n"
                    })
                    .await
                    .context("Failed to toggle repeat")?;
                    updates |= 0b101;
                }
                Command::ToggleRandom => {
                    cl.command(if s.status.random {
                        b"random 0\n"
                    } else {
                        b"random 1\n"
                    })
                    .await
                    .context("Failed to toggle random")?;
                    updates |= 0b101;
                }
                Command::ToggleSingle => {
                    cl.command(if s.status.single == Some(true) {
                        b"single 0\n"
                    } else {
                        b"single 1\n"
                    })
                    .await
                    .context("Failed to toggle single")?;
                    updates |= 0b101;
                }
                Command::ToggleOneshot => {
                    cl.command(
                        s.status
                            .single
                            .map_or(b"single 0\n", |_| b"single oneshot\n"),
                    )
                    .await
                    .context("Failed to toggle oneshot")?;
                    updates |= 0b101;
                }
                Command::ToggleConsume => {
                    cl.command(if s.status.consume {
                        b"consume 0\n"
                    } else {
                        b"consume 1\n"
                    })
                    .await
                    .context("Failed to toggle consume")?;
                    updates |= 0b101;
                }
                Command::TogglePause => {
                    cl.command(s.status.state.map_or(b"play\n", |_| b"pause\n"))
                        .await
                        .context("Failed to toggle pause")?;
                    updates |= 0b101;
                }
                Command::Stop => {
                    cl.command(b"stop\n")
                        .await
                        .context("Failed to stop playing")?;
                    updates |= 0b101;
                }
                Command::SeekBackwards => {
                    cl.command(seek_backwards)
                        .await
                        .context("Failed to seek backwards")?;
                    updates |= 0b101;
                }
                Command::SeekForwards => {
                    cl.command(seek_forwards)
                        .await
                        .context("Failed to seek forwards")?;
                    updates |= 0b101;
                }
                Command::Previous => {
                    cl.command(b"previous\n")
                        .await
                        .context("Failed to play previous song")?;
                    updates |= 0b101;
                }
                Command::Next => {
                    cl.command(b"next\n")
                        .await
                        .context("Failed to play next song")?;
                    updates |= 0b101;
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
                    updates |= 0b101;
                }
                Command::Reselect => {
                    s.selected = s.reselect();
                    s.liststate.select(Some(s.selected));
                    updates |= 0b001;
                }
                Command::Down => {
                    let len = if s.query.is_empty() {
                        s.queue.len()
                    } else {
                        s.filtered.len()
                    };
                    if s.selected >= len {
                        s.selected = s.reselect();
                    } else if s.selected == len - 1 {
                        if cycle {
                            s.selected = 0;
                        }
                    } else {
                        s.selected += 1;
                    }
                    s.liststate.select(Some(s.selected));
                    updates |= 0b001;
                }
                Command::Up => {
                    let len = if s.query.is_empty() {
                        s.queue.len()
                    } else {
                        s.filtered.len()
                    };
                    if s.selected >= len {
                        s.selected = s.reselect();
                    } else if s.selected == 0 {
                        if cycle {
                            s.selected = len - 1;
                        }
                    } else {
                        s.selected -= 1;
                    }
                    s.liststate.select(Some(s.selected));
                    updates |= 0b001;
                }
                Command::JumpDown => {
                    let len = if s.query.is_empty() {
                        s.queue.len()
                    } else {
                        s.filtered.len()
                    };
                    s.selected = if s.selected >= len {
                        s.reselect()
                    } else if cycle {
                        (s.selected + jump_lines) % len
                    } else {
                        min(s.selected + jump_lines, len - 1)
                    };
                    s.liststate.select(Some(s.selected));
                    updates |= 0b001;
                }
                Command::JumpUp => {
                    let len = if s.query.is_empty() {
                        s.queue.len()
                    } else {
                        s.filtered.len()
                    };
                    s.selected = if s.selected >= len {
                        s.reselect()
                    } else if cycle {
                        ((s.selected as isize - jump_lines as isize) % len as isize) as usize
                    } else if s.selected < jump_lines {
                        0
                    } else {
                        s.selected - jump_lines
                    };
                    s.liststate.select(Some(s.selected));
                    updates |= 0b001;
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
                    updates |= 0b001;
                }
                Command::BackspaceSearch => {
                    let c = s.query.pop();
                    if !s.query.is_empty() {
                        s.update_search(&queue_strings);
                    } else if c.is_some() {
                        s.selected = s.reselect();
                        s.liststate.select(Some(s.selected));
                    }
                    updates |= 0b001;
                }
                Command::QuitSearch => {
                    s.quit_search();
                    updates |= 0b001;
                }
                Command::Searching(x) => {
                    s.searching = x;
                    updates |= 0b001;
                }
            },
            _ => empty = true,
        }

        // conditionally update status
        if updates & 0b100 == 0b100 {
            s.status = cl.status().await?;
        }

        // conditionally update queue
        if updates & 0b010 == 0b010 {
            let res = cl.queue(s.status.queue_len, &cfg.search_fields).await?;
            s.queue = res.0;
            queue_strings = res.1;
            s.selected = s.reselect();
            s.liststate.select(None);
            s.liststate.select(Some(s.selected));
            if !s.query.is_empty() {
                s.update_search(&queue_strings);
            }
        }

        // conditionally update frame
        if updates & 0b001 == 0b001 {
            render(&mut term, &cfg.layout, &mut s)?;
        }

        // wait for more commands if none was received
        if empty {
            thread::park();
        }
    }
}
