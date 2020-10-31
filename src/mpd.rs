use anyhow::{bail, Context, Result};
use expand::expand;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};

use std::net::SocketAddr;

use crate::fail;

pub type Client = BufReader<TcpStream>;

#[derive(Debug)]
pub struct Status {
    pub repeat: bool,
    pub random: bool,
    pub single: Option<bool>, // None: oneshot
    pub consume: bool,
    pub song: Option<Song>,
}

#[derive(Copy, Clone, Debug)]
pub struct Song {
    pub pos: usize,
    pub elapsed: u16,
}

#[derive(Debug)]
pub struct Track {
    pub file: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub title: Option<String>,
    pub time: u16,
}

pub async fn init(addr: SocketAddr) -> Result<Client> {
    let mut cl = BufReader::new(
        TcpStream::connect(&addr)
            .await
            .with_context(fail::connect(addr))?,
    );

    let mut buf = [0; 7];
    cl.read(&mut buf).await?;
    if &buf != b"OK MPD " {
        bail!("server did not greet with a success");
    }
    cl.read_line(&mut String::new()).await?;

    Ok(cl)
}

pub async fn idle_playlist(cl: &mut Client) -> Result<()> {
    cl.write_all(b"idle playlist\n").await?;
    let mut lines = cl.lines();

    while let Some(line) = lines.next_line().await? {
        match line.as_bytes() {
            b"OK" => break,
            _ => continue,
        }
    }

    Ok(())
}

pub async fn queue(cl: &mut Client) -> Result<Vec<Track>> {
    let mut first = true;
    let mut tracks = Vec::new();

    let mut file = None;
    let mut artist = None;
    let mut album = None;
    let mut title = None;
    let mut time = None;

    cl.write_all(b"playlistinfo\n").await?;
    let mut lines = cl.lines();

    while let Some(line) = lines.next_line().await? {
        match line.as_bytes() {
            b"OK" => break,
            expand!([@b"file: ", xs @ ..]) => {
                if first {
                    first = false;
                } else {
                    if let (Some(file), Some(time)) = (file, time) {
                        tracks.push(Track {
                            file,
                            artist,
                            album,
                            title,
                            time,
                        });
                    } else {
                        bail!("incomplete playlist response");
                    }
                }

                file = Some(String::from_utf8_lossy(xs).into());
                artist = None;
                album = None;
                title = None;
                time = None;
            }
            expand!([@b"Artist: ", xs @ ..]) => {
                artist = Some(String::from_utf8_lossy(xs).into());
            }
            expand!([@b"Album: ", xs @ ..]) => {
                album = Some(String::from_utf8_lossy(xs).into());
            }
            expand!([@b"Title: ", xs @ ..]) => {
                title = Some(String::from_utf8_lossy(xs).into());
            }
            expand!([@b"Time: ", xs @ ..]) => {
                time = Some(String::from_utf8_lossy(xs).parse()?);
            }
            _ => continue,
        }
    }

    if let (Some(file), Some(time)) = (file, time) {
        tracks.push(Track {
            file,
            artist,
            album,
            title,
            time,
        });
    }

    Ok(tracks)
}

pub async fn status(cl: &mut Client) -> Result<Status> {
    let mut repeat = None;
    let mut random = None;
    let mut single = None;
    let mut consume = None;
    let mut pos = None;
    let mut elapsed = None;

    cl.write_all(b"status\n").await?;
    let mut lines = cl.lines();

    while let Some(line) = lines.next_line().await? {
        match line.as_bytes() {
            b"OK" => break,
            b"repeat: 0" => repeat = Some(false),
            b"repeat: 1" => repeat = Some(true),
            b"random: 0" => random = Some(false),
            b"random: 1" => random = Some(true),
            b"single: 0" => single = Some(Some(false)),
            b"single: 1" => single = Some(Some(true)),
            b"single: oneshot" => single = Some(None),
            b"consume: 0" => consume = Some(false),
            b"consume: 1" => consume = Some(true),
            expand!([@b"song: ", xs @ ..]) => {
                pos = Some(String::from_utf8_lossy(xs).parse()?);
            }
            expand!([@b"elapsed: ", xs @ ..]) => {
                elapsed = Some(String::from_utf8_lossy(xs).parse::<f32>()?.round() as u16);
            }
            _ => continue,
        }
    }

    if let (Some(repeat), Some(random), Some(single), Some(consume)) =
        (repeat, random, single, consume)
    {
        Ok(Status {
            repeat,
            random,
            single,
            consume,
            song: if let (Some(pos), Some(elapsed)) = (pos, elapsed) {
                Some(Song { pos, elapsed })
            } else {
                None
            },
        })
    } else {
        bail!("incomplete status response");
    }
}
