#![feature(async_closure)]
#![forbid(unsafe_code)]

mod fail;
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
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{List, ListItem, Paragraph},
    Terminal,
};

use std::{
    io::{stdout, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    process::exit,
    sync::Arc,
};

use crate::mpd::{Song, Status, Track};

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
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6600);

    let queue_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ]);

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
        let status = (*status.lock().await).clone();

        term.draw(|frame| {
            let len = queue.len();
            let mut titles = Vec::with_capacity(len);
            let mut artists = Vec::with_capacity(len);
            let mut albums = Vec::with_capacity(len);

            for Track {
                title,
                artist,
                album,
                ..
            } in queue
            {
                titles.push(ListItem::new(title.clone().unwrap_or_default()));
                artists.push(ListItem::new(artist.clone().unwrap_or_default()));
                albums.push(ListItem::new(album.clone().unwrap_or_default()));
            }

            let Rect {
                x,
                y,
                width,
                height,
            } = frame.size();
            let chunks = queue_layout.split(Rect {
                x,
                y,
                width,
                height: y + height - 1,
            });
            frame.render_widget(List::new(titles), chunks[0]);
            frame.render_widget(List::new(artists), chunks[1]);
            frame.render_widget(List::new(albums), chunks[2]);

            if let Status {
                song: Some(Song { pos, elapsed }),
                ..
            } = status
            {
                if let Some(Track {
                    file,
                    artist,
                    album,
                    title,
                    time,
                }) = queue.get(pos)
                {
                    frame.render_widget(
                        Paragraph::new(format!(
                            "[{:02}:{:02}/{:02}:{:02}] {}",
                            elapsed / 60,
                            elapsed % 60,
                            time / 60,
                            time % 60,
                            match (title, artist, album) {
                                (Some(title), Some(artist), Some(album)) =>
                                    format!("{} - {} - {}", title, artist, album),
                                (Some(title), Some(artist), _) => format!("{} - {}", title, artist),
                                (Some(title), ..) => title.clone(),
                                _ => file.clone(),
                            }
                        )),
                        Rect {
                            x,
                            y: y + height - 1,
                            width,
                            height: 1,
                        },
                    )
                }
            }
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
