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
    pub state: Option<bool>, // Some(true): play, Some(false): pause, None: stop
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

pub async fn idle(cl: &mut Client) -> Result<(bool, bool)> {
    cl.write_all(b"idle playlist player options\n").await?;
    let mut lines = cl.lines();

    let mut queue = false;
    let mut status = false;

    while let Some(line) = lines.next_line().await? {
        match line.as_bytes() {
            b"changed: playlist" => queue = true,
            b"changed: player" => status = true,
            b"changed: options" => status = true,
            b"OK" => break,
            _ => continue,
        }
    }

    Ok((queue, status))
}

pub async fn queue(cl: &mut Client) -> Result<(Vec<Track>, Vec<String>)> {
    let mut first = true;
    let mut tracks = Vec::new();
    let mut track_strings = Vec::new();

    let mut file: Option<String> = None;
    let mut artist: Option<String> = None;
    let mut album: Option<String> = None;
    let mut title: Option<String> = None;
    let mut time = None;

    cl.write_all(b"playlistinfo\n").await?;
    let mut lines = cl.lines();

    while let Some(line) = lines.next_line().await? {
        match line.as_bytes() {
            b"OK" => break,
            expand!([@b"file: ", xs @ ..]) => {
                if first {
                    first = false;
                } else if let (Some(file), Some(time)) = (file, time) {
                    let mut track_string = file.to_lowercase();
                    if let Some(artist) = &artist {
                        track_string.push('\n');
                        track_string.push_str(&artist.to_lowercase());
                    }
                    if let Some(album) = &album {
                        track_string.push('\n');
                        track_string.push_str(&album.to_lowercase());
                    }
                    if let Some(title) = &title {
                        track_string.push('\n');
                        track_string.push_str(&title.to_lowercase());
                    }
                    track_strings.push(track_string);
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
        let mut track_string = file.to_lowercase();
        if let Some(artist) = &artist {
            track_string.push('\n');
            track_string.push_str(&artist.to_lowercase());
        }
        if let Some(album) = &album {
            track_string.push('\n');
            track_string.push_str(&album.to_lowercase());
        }
        if let Some(title) = &title {
            track_string.push('\n');
            track_string.push_str(&title.to_lowercase());
        }
        track_strings.push(track_string);
        tracks.push(Track {
            file,
            artist,
            album,
            title,
            time,
        });
    }

    Ok((tracks, track_strings))
}

pub async fn status(cl: &mut Client) -> Result<Status> {
    let mut repeat = None;
    let mut random = None;
    let mut single = None;
    let mut consume = None;
    let mut state = None;
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
            b"state: play" => state = Some(true),
            b"state: pause" => state = Some(false),
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
            state,
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

pub async fn play(cl: &mut Client, pos: usize) -> Result<()> {
    cl.write_all(b"play ").await?;
    cl.write_all(pos.to_string().as_bytes()).await?;
    cl.write_u8(b'\n').await?;
    let mut lines = cl.lines();

    while let Some(line) = lines.next_line().await? {
        match line.as_bytes() {
            b"OK" | expand!([@b"ACK ", ..]) => break,
            _ => continue,
        }
    }

    Ok(())
}

pub async fn command(cl: &mut Client, cmd: &[u8]) -> Result<()> {
    cl.write_all(cmd).await?;
    let mut lines = cl.lines();

    while let Some(line) = lines.next_line().await? {
        match line.as_bytes() {
            b"OK" | expand!([@b"ACK ", ..]) => break,
            _ => continue,
        }
    }

    Ok(())
}
