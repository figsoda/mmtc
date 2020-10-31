use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Span, Spans},
    widgets::{List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::{
    config::{Condition, Constrained, Texts, Widget},
    mpd::{Song, Status, Track},
};

pub fn render(
    frame: &mut Frame<impl Backend>,
    size: Rect,
    widget: &Widget,
    queue: &Vec<Track>,
    status: &Status,
    selected: usize,
    liststate: &ListState,
) {
    match widget {
        Widget::Rows(xs) => {
            let len = xs.capacity();
            let mut ws = Vec::with_capacity(len);
            let mut cs = Vec::with_capacity(len);

            let denom = xs.iter().fold(0, |n, x| {
                if let Constrained::Ratio(m, _) = x {
                    n + m
                } else {
                    n
                }
            });

            for x in xs {
                let (w, constraint) = match x {
                    Constrained::Fixed(n, w) => (w, Constraint::Length(*n)),
                    Constrained::Max(n, w) => (w, Constraint::Max(*n)),
                    Constrained::Min(n, w) => (w, Constraint::Min(*n)),
                    Constrained::Ratio(n, w) => (w, Constraint::Ratio(*n, denom)),
                };
                ws.push(w);
                cs.push(constraint);
            }

            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(cs);

            let mut chunks = layout.split(size).into_iter();
            let mut ws = ws.into_iter();

            while let (Some(chunk), Some(w)) = (chunks.next(), ws.next()) {
                render(frame, chunk, w, queue, status, selected, liststate);
            }
        }
        Widget::Columns(xs) => {
            let len = xs.capacity();
            let mut ws = Vec::with_capacity(len);
            let mut cs = Vec::with_capacity(len);

            let denom = xs.iter().fold(0, |n, x| {
                if let Constrained::Ratio(m, _) = x {
                    n + m
                } else {
                    n
                }
            });

            for x in xs {
                let (w, constraint) = match x {
                    Constrained::Fixed(n, w) => (w, Constraint::Length(*n)),
                    Constrained::Max(n, w) => (w, Constraint::Max(*n)),
                    Constrained::Min(n, w) => (w, Constraint::Min(*n)),
                    Constrained::Ratio(n, w) => (w, Constraint::Ratio(*n, denom)),
                };
                ws.push(w);
                cs.push(constraint);
            }

            let layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(cs);

            let mut chunks = layout.split(size).into_iter();
            let mut ws = ws.into_iter();

            while let (Some(chunk), Some(w)) = (chunks.next(), ws.next()) {
                render(frame, chunk, w, queue, status, selected, liststate);
            }
        }
        Widget::Textbox(xss) => {
            let mut spans = Vec::new();
            let current_track = if let Some(Song { pos, .. }) = status.song {
                queue.get(pos)
            } else {
                None
            };
            flatten(&mut spans, &xss, status, current_track, None, false);
            frame.render_widget(Paragraph::new(Spans::from(spans)), size);
        }
        Widget::Queue { columns } => {
            let len = columns.capacity();
            let mut ws = Vec::with_capacity(len);
            let mut cs = Vec::with_capacity(len);

            let denom = columns.iter().fold(0, |n, x| {
                if let Constrained::Ratio(m, _) = x {
                    n + m
                } else {
                    n
                }
            });

            let current_track = if let Some(Song { pos, .. }) = status.song {
                queue.get(pos)
            } else {
                None
            };

            for column in columns {
                let len = queue.len();
                if len == 0 {
                    continue;
                }

                let (xs, constraint) = match column {
                    Constrained::Fixed(n, w) => (w, Constraint::Length(*n)),
                    Constrained::Max(n, w) => (w, Constraint::Max(*n)),
                    Constrained::Min(n, w) => (w, Constraint::Min(*n)),
                    Constrained::Ratio(n, xs) => (xs, Constraint::Ratio(*n, denom)),
                };

                let mut items = Vec::with_capacity(len);
                for i in 0 .. len - 1 {
                    let mut spans = Vec::new();
                    flatten(
                        &mut spans,
                        xs,
                        status,
                        current_track,
                        Some(&queue[i]),
                        i == selected,
                    );
                    items.push(ListItem::new(Spans::from(spans)));
                }
                ws.push(List::new(items));
                cs.push(constraint);
            }

            let layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(cs);

            let mut chunks = layout.split(size).into_iter();
            let mut ws = ws.into_iter();

            while let (Some(chunk), Some(w)) = (chunks.next(), ws.next()) {
                frame.render_stateful_widget(w, chunk, &mut liststate.clone());
            }
        }
    }
}

fn flatten(
    spans: &mut Vec<Span>,
    xs: &Texts,
    status: &Status,
    current_track: Option<&Track>,
    queue_track: Option<&Track>,
    selected: bool,
) {
    match xs {
        Texts::Empty => (),
        Texts::Text(x) => spans.push(Span::raw(x.clone())),
        Texts::CurrentElapsed => {
            if let Some(Song { elapsed, .. }) = status.song {
                spans.push(Span::raw(format!(
                    "{:02}:{:02}",
                    elapsed / 60,
                    elapsed % 60
                )))
            }
        }
        Texts::CurrentDuration => {
            if let Some(Track { time, .. }) = current_track {
                spans.push(Span::raw(format!("{:02}:{:02}", time / 60, time % 60,)))
            }
        }
        Texts::CurrentFile => {
            if let Some(Track { file, .. }) = current_track {
                spans.push(Span::raw(file.clone()));
            }
        }
        Texts::CurrentTitle => {
            if let Some(Track {
                title: Some(title), ..
            }) = current_track
            {
                spans.push(Span::raw(title.clone()));
            }
        }
        Texts::CurrentArtist => {
            if let Some(Track {
                artist: Some(artist),
                ..
            }) = current_track
            {
                spans.push(Span::raw(artist.clone()));
            }
        }
        Texts::CurrentAlbum => {
            if let Some(Track {
                album: Some(album), ..
            }) = current_track
            {
                spans.push(Span::raw(album.clone()));
            }
        }
        Texts::QueueDuration => {
            if let Some(Track { time, .. }) = queue_track {
                spans.push(Span::raw(format!("{:02}:{:02}", time / 60, time % 60,)))
            }
        }
        Texts::QueueFile => {
            if let Some(Track { file, .. }) = current_track {
                spans.push(Span::raw(file.clone()));
            }
        }
        Texts::QueueTitle => {
            if let Some(Track {
                title: Some(title), ..
            }) = queue_track
            {
                spans.push(Span::raw(title.clone()));
            }
        }
        Texts::QueueArtist => {
            if let Some(Track {
                artist: Some(artist),
                ..
            }) = queue_track
            {
                spans.push(Span::raw(artist.clone()));
            }
        }
        Texts::QueueAlbum => {
            if let Some(Track {
                album: Some(album), ..
            }) = queue_track
            {
                spans.push(Span::raw(album.clone()));
            }
        }
        Texts::Parts(xss) => {
            for xs in xss {
                flatten(spans, xs, status, current_track, queue_track, selected);
            }
        }
        Texts::If(cond, box yes, box no) => {
            let xs = if eval_cond(cond, status, current_track, selected) {
                yes
            } else {
                no
            };
            flatten(spans, xs, status, current_track, queue_track, selected);
        }
    }
}

fn eval_cond(
    cond: &Condition,
    status: &Status,
    current_track: Option<&Track>,
    selected: bool,
) -> bool {
    match cond {
        Condition::Playing => current_track.is_some(),
        Condition::Repeat => status.repeat,
        Condition::Random => status.random,
        Condition::Single => status.single == Some(true),
        Condition::Oneshot => status.single == None,
        Condition::Consume => status.consume,
        Condition::TitleExist => matches!(current_track, Some(Track { title: Some(_), .. })),
        Condition::ArtistExist => matches!(
            current_track,
            Some(Track {
                artist: Some(_), ..
            }),
        ),
        Condition::AlbumExist => matches!(current_track, Some(Track { album: Some(_), .. })),
        Condition::Selected => selected,
        Condition::Not(box x) => !eval_cond(x, status, current_track, selected),
        Condition::And(box x, box y) => {
            eval_cond(x, status, current_track, selected)
                && eval_cond(y, status, current_track, selected)
        }
        Condition::Or(box x, box y) => {
            eval_cond(x, status, current_track, selected)
                || eval_cond(y, status, current_track, selected)
        }
        Condition::Xor(box x, box y) => {
            eval_cond(x, status, current_track, selected)
                ^ eval_cond(y, status, current_track, selected)
        }
    }
}
