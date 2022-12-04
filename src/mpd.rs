use anyhow::{bail, Context, Result};
use async_net::{AsyncToSocketAddrs, TcpStream};
use expand::expand;
use futures_lite::{
    io::{split, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, ReadHalf, WriteHalf},
    StreamExt,
};

use std::io::{stdout, Write};

use crate::config::SearchFields;

pub struct Client {
    r: BufReader<ReadHalf<TcpStream>>,
    w: WriteHalf<TcpStream>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum PlayerState {
    Play,
    Pause,
    Stop,
}

#[derive(Debug)]
pub struct Status {
    pub repeat: bool,
    pub random: bool,
    pub single: Option<bool>, // None: oneshot
    pub consume: bool,
    pub queue_len: usize,
    pub state: PlayerState,
    pub song: Option<Song>,
}

#[derive(Debug)]
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
    pub async fn init(addr: impl AsyncToSocketAddrs) -> Result<Client> {
        async move {
            let (r, w) = split(TcpStream::connect(addr).await?);
            let mut cl = Client {
                r: BufReader::new(r),
                w,
            };

            let buf = &mut [0; 7];
            cl.r.read(buf).await?;
            if buf != b"OK MPD " {
                bail!("server did not greet with a success");
            }
            cl.r.read_line(&mut String::with_capacity(8)).await?;

            Ok(cl)
        }
        .await
        .context("Failed to init client")
    }

    pub async fn idle(&mut self) -> Result<(bool, bool)> {
        async move {
            self.w.write_all(b"idle options player playlist\n").await?;
            let mut lines = (&mut self.r).lines();
            let mut status = false;
            let mut queue = false;

            while let Some(line) = lines.next().await {
                match line?.as_bytes() {
                    b"changed: options" => status = true,
                    b"changed: player" => status = true,
                    b"changed: playlist" => queue = true,
                    b"OK" => break,
                    _ => continue,
                }
            }

            Result::<_>::Ok((status, queue))
        }
        .await
        .context("Failed to idle")
    }

    pub async fn queue(
        &mut self,
        len: usize,
        search_fields: &SearchFields,
    ) -> Result<(Vec<Track>, Vec<String>)> {
        async move {
            let mut first = true;
            let mut tracks = Vec::with_capacity(len);
            let mut track_strings = Vec::with_capacity(len);

            let mut file: Option<String> = None;
            let mut artist: Option<String> = None;
            let mut album: Option<String> = None;
            let mut title: Option<String> = None;
            let mut time = None;

            self.w.write_all(b"playlistinfo\n").await?;
            let mut lines = (&mut self.r).lines();

            while let Some(line) = lines.next().await {
                let line = line?;
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

            if let Some(file) = file {
                let track = Track {
                    file,
                    artist,
                    album,
                    title,
                    time: time.unwrap_or_default(),
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
        async move {
            let mut repeat = None;
            let mut random = None;
            let mut single = None;
            let mut consume = None;
            let mut queue_len = None;
            let mut state = PlayerState::Stop;
            let mut pos = None;
            let mut elapsed = None;

            self.w.write_all(b"status\n").await?;
            let mut lines = (&mut self.r).lines();

            while let Some(line) = lines.next().await {
                let line = line?;
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
                    expand!([@b"playlistlength: ", ..]) => queue_len = Some(line[16 ..].parse()?),
                    b"state: play" => state = PlayerState::Play,
                    b"state: pause" => state = PlayerState::Pause,
                    expand!([@b"song: ", ..]) => pos = Some(line[6 ..].parse()?),
                    expand!([@b"elapsed: ", ..]) => {
                        elapsed = Some(line[9 ..].parse::<f32>()?.round() as u16)
                    }
                    _ => continue,
                }
            }

            if let (Some(repeat), Some(random), Some(single), Some(consume), Some(queue_len)) =
                (repeat, random, single, consume, queue_len)
            {
                Ok(Status {
                    repeat,
                    random,
                    single,
                    consume,
                    queue_len,
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
        self.w.write_all(b"play ").await?;
        self.w.write_all(pos.to_string().as_bytes()).await?;
        self.w.write_all(b"\n").await?;
        let mut lines = (&mut self.r).lines();

        while let Some(line) = lines.next().await {
            match line?.as_bytes() {
                b"OK" | expand!([@b"ACK ", ..]) => break,
                _ => continue,
            }
        }

        Ok(())
    }

    pub async fn command(&mut self, cmd: &[u8]) -> Result<()> {
        self.w.write_all(cmd).await?;
        self.w.write_all(b"\n").await?;
        let mut lines = (&mut self.r).lines();

        while let Some(line) = lines.next().await {
            match line?.as_bytes() {
                b"OK" | expand!([@b"ACK ", ..]) => break,
                _ => continue,
            }
        }

        Ok(())
    }

    pub async fn command_stdout(&mut self, cmd: &[u8]) -> Result<()> {
        self.w.write_all(cmd).await?;
        self.w.write_all(b"\n").await?;

        let mut stdout = stdout().lock();
        let mut lines = (&mut self.r).lines();

        while let Some(line) = lines.next().await {
            let line = line?;
            let line = line.as_bytes();

            stdout.write_all(line)?;
            stdout.write_all(b"\n")?;
            match line {
                b"OK" | expand!([@b"ACK ", ..]) => break,
                _ => continue,
            }
        }

        Ok(())
    }
}
