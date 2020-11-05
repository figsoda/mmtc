use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Span, Spans},
    widgets::{List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::{
    config::{AddStyle, Column, Condition, Constrained, Texts, Widget},
    mpd::{Song, Status, Track},
};

struct FlattenState<'a> {
    status: &'a Status,
    current_track: Option<&'a Track>,
    queue_track: Option<&'a Track>,
    queue_current: bool,
    selected: bool,
    searching: bool,
    query: &'a str,
    style: Style,
}

struct ConditionState<'a> {
    status: &'a Status,
    current_track: Option<&'a Track>,
    queue_current: bool,
    selected: bool,
    searching: bool,
    query: &'a str,
}

pub fn render(
    frame: &mut Frame<impl Backend>,
    size: Rect,
    widget: &Widget,
    queue: &[Track],
    searching: bool,
    query: &str,
    filtered: &[usize],
    status: &Status,
    liststate: &mut ListState,
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
                render(
                    frame, chunk, w, queue, searching, query, filtered, status, liststate,
                );
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
                render(
                    frame, chunk, w, queue, searching, query, filtered, status, liststate,
                );
            }
        }
        Widget::Textbox(xs) => {
            let mut spans = Vec::new();
            flatten(
                &mut spans,
                &xs,
                &FlattenState {
                    status,
                    current_track: if let Some(Song { pos, .. }) = status.song {
                        queue.get(pos)
                    } else {
                        None
                    },
                    queue_track: None,
                    queue_current: false,
                    selected: false,
                    searching,
                    query,
                    style: Style::default(),
                },
            );
            frame.render_widget(Paragraph::new(Spans::from(spans)), size);
        }
        Widget::TextboxC(xs) => {
            let mut spans = Vec::new();
            flatten(
                &mut spans,
                &xs,
                &FlattenState {
                    status,
                    current_track: if let Some(Song { pos, .. }) = status.song {
                        queue.get(pos)
                    } else {
                        None
                    },
                    queue_track: None,
                    queue_current: false,
                    selected: false,
                    searching,
                    query,
                    style: Style::default(),
                },
            );
            frame.render_widget(
                Paragraph::new(Spans::from(spans)).alignment(Alignment::Center),
                size,
            );
        }
        Widget::TextboxR(xs) => {
            let mut spans = Vec::new();
            flatten(
                &mut spans,
                &xs,
                &FlattenState {
                    status,
                    current_track: if let Some(Song { pos, .. }) = status.song {
                        queue.get(pos)
                    } else {
                        None
                    },
                    queue_track: None,
                    queue_current: false,
                    selected: false,
                    searching,
                    query,
                    style: Style::default(),
                },
            );
            frame.render_widget(
                Paragraph::new(Spans::from(spans)).alignment(Alignment::Right),
                size,
            );
        }
        Widget::Queue(xs) => {
            let len = xs.capacity();
            let mut ws = Vec::with_capacity(len);
            let mut cs = Vec::with_capacity(len);

            let denom = xs.iter().fold(0, |n, Column { item, .. }| {
                if let Constrained::Ratio(m, _) = item {
                    n + m
                } else {
                    n
                }
            });

            let (pos, current_track) = if let Some(Song { pos, .. }) = status.song {
                (Some(pos), queue.get(pos))
            } else {
                (None, None)
            };

            for column in xs {
                let len = queue.len();
                if len == 0 {
                    continue;
                }

                let (txts, constraint) = match &column.item {
                    Constrained::Fixed(n, txts) => (txts, Constraint::Length(*n)),
                    Constrained::Max(n, txts) => (txts, Constraint::Max(*n)),
                    Constrained::Min(n, txts) => (txts, Constraint::Min(*n)),
                    Constrained::Ratio(n, txts) => (txts, Constraint::Ratio(*n, denom)),
                };

                let mut items = Vec::with_capacity(len);
                if query.is_empty() {
                    for (i, track) in queue.iter().enumerate() {
                        let mut spans = Vec::new();
                        flatten(
                            &mut spans,
                            txts,
                            &FlattenState {
                                status,
                                current_track,
                                queue_track: Some(track),
                                queue_current: pos == Some(i),
                                selected: liststate.selected() == Some(i),
                                searching,
                                query,
                                style: Style::default(),
                            },
                        );
                        items.push(ListItem::new(Spans::from(spans)));
                    }
                } else {
                    for &i in filtered {
                        let mut spans = Vec::new();
                        flatten(
                            &mut spans,
                            txts,
                            &FlattenState {
                                status,
                                current_track,
                                queue_track: Some(&queue[i]),
                                queue_current: pos == Some(i),
                                selected: liststate.selected() == Some(i),
                                searching,
                                query,
                                style: Style::default(),
                            },
                        );
                        items.push(ListItem::new(Spans::from(spans)));
                    }
                }
                ws.push(
                    List::new(items)
                        .style(patch_style(Style::default(), &column.style))
                        .highlight_style(patch_style(Style::default(), &column.selected_style)),
                );
                cs.push(constraint);
            }

            let layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(cs);

            let mut chunks = layout.split(size).into_iter();
            let mut ws = ws.into_iter();

            if let (Some(chunk), Some(w)) = (chunks.next(), ws.next()) {
                frame.render_stateful_widget(w, chunk, liststate);
                while let (Some(chunk), Some(w)) = (chunks.next(), ws.next()) {
                    frame.render_stateful_widget(w, chunk, &mut liststate.clone());
                }
            }
        }
    }
}

fn flatten(spans: &mut Vec<Span>, xs: &Texts, s: &FlattenState) {
    match xs {
        Texts::Text(x) => spans.push(Span::styled(x.clone(), s.style)),
        Texts::CurrentElapsed => {
            if let Some(Song { elapsed, .. }) = s.status.song {
                spans.push(Span::styled(
                    format!("{}:{:02}", elapsed / 60, elapsed % 60),
                    s.style,
                ));
            }
        }
        Texts::CurrentDuration => {
            if let Some(Track { time, .. }) = s.current_track {
                spans.push(Span::styled(
                    format!("{}:{:02}", time / 60, time % 60),
                    s.style,
                ));
            }
        }
        Texts::CurrentFile => {
            if let Some(Track { file, .. }) = s.current_track {
                spans.push(Span::styled(file.clone(), s.style));
            }
        }
        Texts::CurrentTitle => {
            if let Some(Track {
                title: Some(title), ..
            }) = s.current_track
            {
                spans.push(Span::styled(title.clone(), s.style));
            }
        }
        Texts::CurrentArtist => {
            if let Some(Track {
                artist: Some(artist),
                ..
            }) = s.current_track
            {
                spans.push(Span::styled(artist.clone(), s.style));
            }
        }
        Texts::CurrentAlbum => {
            if let Some(Track {
                album: Some(album), ..
            }) = s.current_track
            {
                spans.push(Span::styled(album.clone(), s.style));
            }
        }
        Texts::QueueDuration => {
            if let Some(Track { time, .. }) = s.queue_track {
                spans.push(Span::styled(
                    format!("{}:{:02}", time / 60, time % 60),
                    s.style,
                ));
            }
        }
        Texts::QueueFile => {
            if let Some(Track { file, .. }) = s.current_track {
                spans.push(Span::styled(file.clone(), s.style));
            }
        }
        Texts::QueueTitle => {
            if let Some(Track {
                title: Some(title), ..
            }) = s.queue_track
            {
                spans.push(Span::styled(title.clone(), s.style));
            }
        }
        Texts::QueueArtist => {
            if let Some(Track {
                artist: Some(artist),
                ..
            }) = s.queue_track
            {
                spans.push(Span::styled(artist.clone(), s.style));
            }
        }
        Texts::QueueAlbum => {
            if let Some(Track {
                album: Some(album), ..
            }) = s.queue_track
            {
                spans.push(Span::styled(album.clone(), s.style));
            }
        }
        Texts::Query => {
            spans.push(Span::styled(String::from(s.query), s.style));
        }
        Texts::Styled(styles, box xs) => {
            flatten(
                spans,
                xs,
                &FlattenState {
                    style: patch_style(s.style, styles),
                    ..*s
                },
            );
        }
        Texts::Parts(xss) => {
            for xs in xss {
                flatten(spans, xs, s);
            }
        }
        Texts::If(cond, box xs, Some(box ys)) => {
            flatten(
                spans,
                if eval_cond(
                    cond,
                    &ConditionState {
                        status: s.status,
                        current_track: s.current_track,
                        queue_current: s.queue_current,
                        selected: s.selected,
                        searching: s.searching,
                        query: s.query,
                    },
                ) {
                    xs
                } else {
                    ys
                },
                s,
            );
        }
        Texts::If(cond, box xs, None) => {
            if eval_cond(
                cond,
                &ConditionState {
                    status: s.status,
                    current_track: s.current_track,
                    queue_current: s.queue_current,
                    selected: s.selected,
                    searching: s.searching,
                    query: s.query,
                },
            ) {
                flatten(spans, xs, s);
            }
        }
    }
}

fn patch_style(style: Style, styles: &[AddStyle]) -> Style {
    let mut style = style;
    for add_style in styles {
        match add_style {
            AddStyle::Fg(color) => {
                style.fg = Some(*color);
            }
            AddStyle::Bg(color) => {
                style.bg = Some(*color);
            }
            AddStyle::Bold => {
                style = style.add_modifier(Modifier::BOLD);
            }
            AddStyle::NoBold => {
                style = style.remove_modifier(Modifier::BOLD);
            }
            AddStyle::Dim => {
                style = style.add_modifier(Modifier::DIM);
            }
            AddStyle::NoDim => {
                style = style.remove_modifier(Modifier::DIM);
            }
            AddStyle::Italic => {
                style = style.add_modifier(Modifier::ITALIC);
            }
            AddStyle::NoItalic => {
                style = style.remove_modifier(Modifier::ITALIC);
            }
            AddStyle::Underlined => {
                style = style.add_modifier(Modifier::UNDERLINED);
            }
            AddStyle::NoUnderlined => {
                style = style.remove_modifier(Modifier::UNDERLINED);
            }
            AddStyle::SlowBlink => {
                style = style.add_modifier(Modifier::SLOW_BLINK);
            }
            AddStyle::NoSlowBlink => {
                style = style.remove_modifier(Modifier::SLOW_BLINK);
            }
            AddStyle::RapidBlink => {
                style = style.add_modifier(Modifier::RAPID_BLINK);
            }
            AddStyle::NoRapidBlink => {
                style = style.remove_modifier(Modifier::RAPID_BLINK);
            }
            AddStyle::Reversed => {
                style = style.add_modifier(Modifier::REVERSED);
            }
            AddStyle::NoReversed => {
                style = style.remove_modifier(Modifier::REVERSED);
            }
            AddStyle::Hidden => {
                style = style.add_modifier(Modifier::HIDDEN);
            }
            AddStyle::NoHidden => {
                style = style.remove_modifier(Modifier::HIDDEN);
            }
            AddStyle::CrossedOut => {
                style = style.add_modifier(Modifier::CROSSED_OUT);
            }
            AddStyle::NoCrossedOut => {
                style = style.remove_modifier(Modifier::CROSSED_OUT);
            }
        }
    }
    style
}

fn eval_cond(cond: &Condition, s: &ConditionState) -> bool {
    match cond {
        Condition::Repeat => s.status.repeat,
        Condition::Random => s.status.random,
        Condition::Single => s.status.single == Some(true),
        Condition::Oneshot => s.status.single == None,
        Condition::Consume => s.status.consume,
        Condition::Playing => s.status.state == Some(true),
        Condition::Paused => s.status.state == Some(false),
        Condition::Stopped => s.status.state == None,
        Condition::TitleExist => matches!(s.current_track, Some(Track { title: Some(_), .. })),
        Condition::ArtistExist => matches!(
            s.current_track,
            Some(Track {
                artist: Some(_), ..
            }),
        ),
        Condition::AlbumExist => matches!(s.current_track, Some(Track { album: Some(_), .. })),
        Condition::QueueCurrent => s.queue_current,
        Condition::Selected => s.selected,
        Condition::Searching => s.searching,
        Condition::Filtered => !s.query.is_empty(),
        Condition::Not(box x) => !eval_cond(x, s),
        Condition::And(box x, box y) => eval_cond(x, s) && eval_cond(y, s),
        Condition::Or(box x, box y) => eval_cond(x, s) || eval_cond(y, s),
        Condition::Xor(box x, box y) => eval_cond(x, s) ^ eval_cond(y, s),
    }
}
