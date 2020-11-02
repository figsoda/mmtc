#![feature(box_patterns)]
#![forbid(unsafe_code)]

mod config;
mod fail;
mod layout;
mod mpd;

use anyhow::{bail, Context, Error, Result};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, MouseEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use dirs::config_dir;
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

/// Minimal mpd terminal client https://github.com/figsoda/mmtc
#[derive(StructOpt)]
#[structopt(
    name = "mmtc",
    rename_all = "kebab-case",
    global_setting = AppSettings::ColoredHelp,
)]
struct Opts {
    /// Specify the config file
    #[structopt(short, long)]
    config: Option<String>,

    /// Specify the address of the mpd server
    #[structopt(short, long)]
    address: Option<String>,

    /// Cycle through the queue
    #[structopt(short, long)]
    cycle: Option<bool>,

    /// The number of lines to jump
    #[structopt(short, long)]
    jump_lines: Option<usize>,

    /// The time in seconds to seek
    #[structopt(short, long)]
    seek_secs: Option<f64>,

    /// The amount of status updates per second
    #[structopt(short, long)]
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
}

fn cleanup() -> Result<()> {
    disable_raw_mode().context("Failed to clean up terminal")?;
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
    if let Err(e) = cleanup().and_then(|_| res) {
        eprintln!("{:?}", e);
        exit(1);
    }
    exit(0);
}

async fn run() -> Result<()> {
    let opts = Opts::from_args();

    let cfg: Config = if let Some(cfg_file) = opts.config {
        ron::from_str(&fs::read_to_string(&cfg_file).with_context(fail::read(&cfg_file))?)
            .with_context(fail::parse_cfg(&cfg_file))?
    } else if let Some(xs) = config_dir() {
        let mut xs = xs;
        xs.push("mmtc");
        xs.push("mmtc.ron");
        ron::from_str(&fs::read_to_string(&xs).with_context(fail::read(xs.display()))?)
            .with_context(fail::parse_cfg(xs.display()))?
    } else {
        bail!("Failed to find the config directory, please specify a config file");
    };

    let addr = if let Some(addr) = opts.address {
        addr.parse().with_context(fail::parse_addr(addr))?
    } else {
        cfg.address
    };
    let cycle = opts.cycle.unwrap_or(cfg.cycle);
    let jump_lines = opts.jump_lines.unwrap_or(cfg.jump_lines);
    let seek_secs = opts.seek_secs.unwrap_or(cfg.seek_secs);

    let mut idle_cl = mpd::init(addr).await?;
    let mut cl = mpd::init(addr).await?;

    let mut queue = mpd::queue(&mut idle_cl).await?;
    let mut status = mpd::status(&mut cl).await?;
    let mut selected = status.song.map_or(0, |song| song.pos);
    let mut liststate = ListState::default();
    liststate.select(Some(selected));

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
            match ev {
                Event::Key(KeyEvent { code, .. }) => match code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        tx.send(Command::Quit).await.unwrap_or_else(die);
                    }
                    KeyCode::Char('r') => {
                        tx.send(Command::ToggleRepeat).await.unwrap_or_else(die);
                    }
                    KeyCode::Char('R') => {
                        tx.send(Command::ToggleRandom).await.unwrap_or_else(die);
                    }
                    KeyCode::Char('s') => {
                        tx.send(Command::ToggleSingle).await.unwrap_or_else(die);
                    }
                    KeyCode::Char('S') => {
                        tx.send(Command::ToggleOneshot).await.unwrap_or_else(die);
                    }
                    KeyCode::Char('c') => {
                        tx.send(Command::ToggleConsume).await.unwrap_or_else(die);
                    }
                    KeyCode::Char('p') => {
                        tx.send(Command::TogglePause).await.unwrap_or_else(die);
                    }
                    KeyCode::Char('h') | KeyCode::Left => {
                        tx.send(Command::SeekBackwards).await.unwrap_or_else(die);
                    }
                    KeyCode::Char('l') | KeyCode::Right => {
                        tx.send(Command::SeekForwards).await.unwrap_or_else(die);
                    }
                    KeyCode::Char('H') => {
                        tx.send(Command::Previous).await.unwrap_or_else(die);
                    }
                    KeyCode::Char('L') => {
                        tx.send(Command::Next).await.unwrap_or_else(die);
                    }
                    KeyCode::Enter => {
                        tx.send(Command::Play).await.unwrap_or_else(die);
                    }
                    KeyCode::Char(' ') => {
                        tx.send(Command::Reselect).await.unwrap_or_else(die);
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        tx.send(Command::Down).await.unwrap_or_else(die);
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        tx.send(Command::Up).await.unwrap_or_else(die);
                    }
                    KeyCode::Char('J') | KeyCode::PageDown => {
                        tx.send(Command::JumpDown).await.unwrap_or_else(die);
                    }
                    KeyCode::Char('K') | KeyCode::PageUp => {
                        tx.send(Command::JumpUp).await.unwrap_or_else(die);
                    }
                    _ => (),
                },
                Event::Mouse(MouseEvent::ScrollDown(..)) => {
                    tx.send(Command::Down).await.unwrap_or_else(die);
                }
                Event::Mouse(MouseEvent::ScrollUp(..)) => {
                    tx.send(Command::Up).await.unwrap_or_else(die);
                }
                Event::Resize(..) => {
                    tx.send(Command::UpdateFrame).await.unwrap_or_else(die);
                }
                _ => (),
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
                        &status,
                        selected,
                        &mut liststate,
                    );
                })
                .context("Failed to draw to terminal")?,
            Command::UpdateQueue => {
                queue = mpd::queue(&mut cl)
                    .await
                    .context("Failed to query queue")
                    .unwrap_or_else(die);
                selected = status.song.map_or(0, |song| song.pos);
                liststate = ListState::default();
                liststate.select(Some(selected));
            }
            Command::UpdateStatus => {
                status = mpd::status(&mut cl)
                    .await
                    .context("Failed to query status")
                    .unwrap_or_else(die);
            }
            Command::ToggleRepeat => {
                mpd::command(
                    &mut cl,
                    if status.repeat {
                        b"repeat 0\n"
                    } else {
                        b"repeat 1\n"
                    },
                )
                .await
                .context("Failed to toggle repeat")
                .unwrap_or_else(die);
                tx.send(Command::UpdateStatus).await?;
                tx.send(Command::UpdateFrame).await?;
            }
            Command::ToggleRandom => {
                mpd::command(
                    &mut cl,
                    if status.random {
                        b"random 0\n"
                    } else {
                        b"random 1\n"
                    },
                )
                .await
                .context("Failed to toggle random")
                .unwrap_or_else(die);
                tx.send(Command::UpdateStatus).await?;
                tx.send(Command::UpdateFrame).await?;
            }
            Command::ToggleSingle => {
                mpd::command(
                    &mut cl,
                    if status.single == Some(true) {
                        b"single 0\n"
                    } else {
                        b"single 1\n"
                    },
                )
                .await
                .context("Failed to toggle single")
                .unwrap_or_else(die);
                tx.send(Command::UpdateStatus).await?;
                tx.send(Command::UpdateFrame).await?;
            }
            Command::ToggleOneshot => {
                mpd::command(
                    &mut cl,
                    if status.single == None {
                        b"single 0\n"
                    } else {
                        b"single oneshot\n"
                    },
                )
                .await
                .context("Failed to toggle oneshot")
                .unwrap_or_else(die);
                tx.send(Command::UpdateStatus).await?;
                tx.send(Command::UpdateFrame).await?;
            }
            Command::ToggleConsume => {
                mpd::command(
                    &mut cl,
                    if status.consume {
                        b"consume 0\n"
                    } else {
                        b"consume 1\n"
                    },
                )
                .await
                .context("Failed to toggle consume")
                .unwrap_or_else(die);
                tx.send(Command::UpdateStatus).await?;
                tx.send(Command::UpdateFrame).await?;
            }
            Command::SeekBackwards => {
                mpd::command(&mut cl, seek_backwards)
                    .await
                    .context("Failed to seek backwards")
                    .unwrap_or_else(die);
                tx.send(Command::UpdateStatus).await?;
                tx.send(Command::UpdateFrame).await?;
            }
            Command::SeekForwards => {
                mpd::command(&mut cl, seek_forwards)
                    .await
                    .context("Failed to seek forwards")
                    .unwrap_or_else(die);
                tx.send(Command::UpdateStatus).await?;
                tx.send(Command::UpdateFrame).await?;
            }
            Command::TogglePause => {
                mpd::command(&mut cl, b"pause\n")
                    .await
                    .context("Failed to toggle pause")
                    .unwrap_or_else(die);
                tx.send(Command::UpdateStatus).await?;
                tx.send(Command::UpdateFrame).await?;
            }
            Command::Previous => {
                mpd::command(&mut cl, b"previous\n")
                    .await
                    .context("Failed to play previous song")
                    .unwrap_or_else(die);
                tx.send(Command::UpdateStatus).await?;
                tx.send(Command::UpdateFrame).await?;
            }
            Command::Next => {
                mpd::command(&mut cl, b"next\n")
                    .await
                    .context("Failed to play previous song")
                    .unwrap_or_else(die);
                tx.send(Command::UpdateStatus).await?;
                tx.send(Command::UpdateFrame).await?;
            }
            Command::Play => {
                if selected < queue.len() {
                    mpd::play(&mut cl, selected)
                        .await
                        .context("Failed to play the selected song")
                        .unwrap_or_else(die);
                }
                tx.send(Command::UpdateStatus).await?;
                tx.send(Command::UpdateFrame).await?;
            }
            Command::Reselect => {
                selected = status.song.map_or(0, |song| song.pos);
                liststate.select(Some(selected));
                tx.send(Command::UpdateFrame).await?;
            }
            Command::Down => {
                let len = queue.len();
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
                let len = queue.len();
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
                let len = queue.len();
                if selected >= len {
                    selected = status.song.map_or(0, |song| song.pos);
                } else {
                    selected = if cycle {
                        (selected + jump_lines) % len
                    } else {
                        min(selected + jump_lines, len - 1)
                    }
                }
                liststate.select(Some(selected));
                tx.send(Command::UpdateFrame).await?;
            }
            Command::JumpUp => {
                let len = queue.len();
                if selected >= len {
                    selected = status.song.map_or(0, |song| song.pos);
                } else {
                    selected = if cycle {
                        (selected as isize - jump_lines as isize) % len as isize
                    } else {
                        max(selected as isize - jump_lines as isize, 0)
                    } as usize
                }
                liststate.select(Some(selected));
                tx.send(Command::UpdateFrame).await?;
            }
        }
    }

    Ok(())
}
