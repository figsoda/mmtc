use anyhow::{bail, Context, Result};
use expand::expand;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};

use std::net::SocketAddr;

use crate::{config::SearchFields, fail};

pub struct Client(BufReader<TcpStream>);

#[derive(Debug)]
pub struct Status {
    pub repeat: bool,
    pub random: bool,
    pub single: Option<bool>, // None: oneshot
    pub consume: bool,
    pub state: Option<bool>, // Some(true): play, Some(false): pause, None: stop
    pub song: Option<Song>,
}

#[derive(Clone, Copy, Debug)]
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

fn track_string(track: &Track, search_fields: &SearchFields) -> String {
    let mut track_string = String::with_capacity(64);

    if search_fields.file {
        track_string.push_str(&track.file.to_lowercase());
        track_string.push('\n');
    }

    if search_fields.title {
        if let Some(title) = &track.title {
            track_string.push_str(&title.to_lowercase());
            track_string.push('\n');
        }
    }

    if search_fields.artist {
        if let Some(artist) = &track.artist {
            track_string.push_str(&artist.to_lowercase());
            track_string.push('\n');
        }
    }

    if search_fields.album {
        if let Some(album) = &track.album {
            track_string.push_str(&album.to_lowercase());
        }
    }

    track_string
}

impl Client {
    pub async fn init(addr: &SocketAddr) -> Result<Client> {
        async {
            let mut cl = BufReader::new(
                TcpStream::connect(addr)
                    .await
                    .with_context(fail::connect(addr))?,
            );

            let buf = &mut [0; 7];
            cl.read(buf).await?;
            if buf != b"OK MPD " {
                bail!("server did not greet with a success");
            }
            cl.read_line(&mut String::new()).await?;

            Ok(Client(cl))
        }
        .await
        .context("Failed to init client")
    }

    pub async fn idle(&mut self) -> Result<(bool, bool)> {
        async {
            let cl = &mut self.0;

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

            Ok((queue, status)) as tokio::io::Result<_>
        }
        .await
        .context("Failed to idle")
    }

    pub async fn queue(
        &mut self,
        search_fields: &SearchFields,
    ) -> Result<(Vec<Track>, Vec<String>)> {
        async {
            let cl = &mut self.0;

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
                    expand!([@b"file: ", ..]) => {
                        if first {
                            first = false;
                        } else if let (Some(file), Some(time)) = (file, time) {
                            let track = Track {
                                file,
                                artist,
                                album,
                                title,
                                time,
                            };
                            track_strings.push(track_string(&track, search_fields));
                            tracks.push(track);
                        } else {
                            bail!("incomplete playlist response");
                        }

                        file = Some(line[6 ..].into());
                        artist = None;
                        album = None;
                        title = None;
                        time = None;
                    }
                    expand!([@b"Artist: ", ..]) => artist = Some(line[8 ..].into()),
                    expand!([@b"Album: ", ..]) => album = Some(line[7 ..].into()),
                    expand!([@b"Title: ", ..]) => title = Some(line[7 ..].into()),
                    expand!([@b"Time: ", ..]) => time = Some(line[6 ..].parse()?),
                    _ => continue,
                }
            }

            if let (Some(file), Some(time)) = (file, time) {
                let track = Track {
                    file,
                    artist,
                    album,
                    title,
                    time,
                };
                track_strings.push(track_string(&track, search_fields));
                tracks.push(track);
            }

            Ok((tracks, track_strings))
        }
        .await
        .context("Failed to query queue")
    }

    pub async fn status(&mut self) -> Result<Status> {
        async {
            let cl = &mut self.0;

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
                    expand!([@b"song: ", ..]) => pos = Some(line[6 ..].parse()?),
                    expand!([@b"elapsed: ", ..]) => {
                        elapsed = Some(line[9 ..].parse::<f32>()?.round() as u16)
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
        .await
        .context("Failed to query status")
    }

    pub async fn play(&mut self, pos: usize) -> Result<()> {
        let cl = &mut self.0;

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

    pub async fn command(&mut self, cmd: &[u8]) -> Result<()> {
        let cl = &mut self.0;

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
}
