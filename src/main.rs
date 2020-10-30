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
    sync::Mutex,
    time::{sleep_until, Duration, Instant},
};
use tui::{backend::CrosstermBackend, Terminal};

use std::{
    io::{stdout, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    process::exit,
    sync::Arc,
};

use crate::config::Config;

fn cleanup() -> Result<()> {
    disable_raw_mode().context("Failed to clean up terminal")?;
    execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture)
        .context("Failed to clean up terminal")?;
    Ok(())
}

fn die<T>(e: impl std::fmt::Display) -> T {
    if let Err(e) = cleanup() {
        eprintln!("{}", e);
    };
    eprintln!("{}", e);
    exit(1);
}

#[tokio::main]
async fn main() -> Result<()> {
    let res = run().await;
    cleanup()?;
    res
}

async fn run() -> Result<()> {
    let cfg: Config = ron::from_str(&std::fs::read_to_string("mmtc.ron").unwrap()).unwrap();
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6600);

    let mut idle_cl = mpd::init(addr).await.with_context(fail::connect(addr))?;
    let mut status_cl = mpd::init(addr).await.with_context(fail::connect(addr))?;

    let queue = Arc::new(Mutex::new(mpd::queue(&mut idle_cl).await?));
    let queue1 = Arc::clone(&queue);
    let status = Arc::new(Mutex::new(mpd::status(&mut status_cl).await?));
    let status1 = Arc::clone(&status);

    tokio::spawn(async move {
        loop {
            mpd::idle_playlist(&mut idle_cl)
                .await
                .context("Failed to idle")
                .unwrap_or_else(die);
            *queue1.lock().await = mpd::queue(&mut idle_cl)
                .await
                .context("Failed to query queue information")
                .unwrap_or_else(die);
        }
    });

    tokio::spawn(async move {
        loop {
            let deadline = Instant::now() + Duration::from_millis(250);
            *status1.lock().await = mpd::status(&mut status_cl)
                .await
                .context("Failed to query status")
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

    loop {
        let deadline = Instant::now() + Duration::from_secs_f32(1.0 / 30.0);

        let queue = &*queue.lock().await;
        let status = &*status.lock().await;

        term.draw(|frame| {
            layout::render(frame, frame.size(), &cfg.layout, queue, status);
        })
        .context("Failed to draw to terminal")?;

        while event::poll(Duration::new(0, 0)).context("Failed to poll events")? {
            match event::read().context("Failed to read events")? {
                Event::Key(KeyEvent { code, .. }) => match code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        cleanup()?;
                        return Ok(());
                    }
                    _ => (),
                },
                _ => (),
            }
        }

        sleep_until(deadline).await;
    }
}
