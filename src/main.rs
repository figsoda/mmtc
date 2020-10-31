#![feature(async_closure)]
#![feature(box_patterns)]
#![forbid(unsafe_code)]

mod config;
mod fail;
mod layout;
mod mpd;

use anyhow::{Context, Result};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tokio::{
    sync::mpsc,
    time::{sleep_until, Duration, Instant},
};
use tui::{backend::CrosstermBackend, Terminal};

use std::{
    fmt::Display,
    io::{stdout, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    process::exit,
};

use crate::{
    config::Config,
    mpd::{Status, Track},
};

fn cleanup() -> Result<()> {
    disable_raw_mode().context("Failed to clean up terminal")?;
    execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture)
        .context("Failed to clean up terminal")?;
    Ok(())
}

fn die<T>(e: impl Display) -> T {
    if let Err(e) = cleanup() {
        eprintln!("{}", e);
    };
    eprintln!("{}", e);
    exit(1);
}

#[derive(Debug)]
enum Command {
    Quit,
    UpdateFrame,
    UpdateQueue(Vec<Track>),
    UpdateStatus(Status),
}

#[tokio::main]
async fn main() -> Result<()> {
    let res = run().await;
    cleanup().and_then(|_| res).map_or_else(Err, |_| exit(0))
}

async fn run() -> Result<()> {
    let cfg: Config = ron::from_str(&std::fs::read_to_string("mmtc.ron").unwrap()).unwrap();
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6600);

    let mut idle_cl = mpd::init(addr).await?;
    let mut status_cl = mpd::init(addr).await?;

    let mut queue = mpd::queue(&mut idle_cl).await?;
    let mut status = mpd::status(&mut status_cl).await?;

    let update_interval = Duration::from_secs_f64(1.0 / cfg.ups);

    let (tx, mut rx) = mpsc::channel(32);
    let tx1 = tx.clone();
    let tx2 = tx.clone();
    let tx3 = tx.clone();

    tokio::spawn(async move {
        let tx = tx1;
        loop {
            mpd::idle_playlist(&mut idle_cl)
                .await
                .context("Failed to idle")
                .unwrap_or_else(die);
            tx.send(Command::UpdateQueue(
                mpd::queue(&mut idle_cl)
                    .await
                    .context("Failed to query queue information")
                    .unwrap_or_else(die),
            ))
            .await
            .unwrap_or_else(die);
        }
    });

    tokio::spawn(async move {
        let tx = tx2;
        loop {
            let deadline = Instant::now() + update_interval;
            tx.send(Command::UpdateStatus(
                mpd::status(&mut status_cl)
                    .await
                    .context("Failed to query status")
                    .unwrap_or_else(die),
            ))
            .await
            .unwrap_or_else(die);
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
                        tx.send(Command::Quit).await.unwrap_or_else(die)
                    }
                    _ => (),
                },
                Event::Resize(..) => tx.send(Command::UpdateFrame).await.unwrap_or_else(die),
                _ => (),
            }
        }
    });

    while let Some(cmd) = rx.recv().await {
        match cmd {
            Command::Quit => return Ok(()),
            Command::UpdateFrame => term
                .draw(|frame| {
                    layout::render(frame, frame.size(), &cfg.layout, &queue, &status);
                })
                .context("Failed to draw to terminal")?,
            Command::UpdateQueue(new_queue) => {
                queue = new_queue;
                tx.send(Command::UpdateFrame).await?;
            }
            Command::UpdateStatus(new_status) => {
                status = new_status;
                tx.send(Command::UpdateFrame).await?;
            }
        }
    }

    Ok(())
}
