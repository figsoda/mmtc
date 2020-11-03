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

pub fn render(
    frame: &mut Frame<impl Backend>,
    size: Rect,
    widget: &Widget,
    queue: &[Track],
    status: &Status,
    selected: usize,
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
        Widget::Textbox(xs) => {
            let mut spans = Vec::new();
            flatten(
                &mut spans,
                &xs,
                status,
                if let Some(Song { pos, .. }) = status.song {
                    queue.get(pos)
                } else {
                    None
                },
                None,
                false,
                false,
                Style::default(),
            );
            frame.render_widget(Paragraph::new(Spans::from(spans)), size);
        }
        Widget::TextboxC(xs) => {
            let mut spans = Vec::new();
            flatten(
                &mut spans,
                &xs,
                status,
                if let Some(Song { pos, .. }) = status.song {
                    queue.get(pos)
                } else {
                    None
                },
                None,
                false,
                false,
                Style::default(),
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
                status,
                if let Some(Song { pos, .. }) = status.song {
                    queue.get(pos)
                } else {
                    None
                },
                None,
                false,
                false,
                Style::default(),
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
                for (i, track) in queue.iter().enumerate() {
                    let mut spans = Vec::new();
                    flatten(
                        &mut spans,
                        txts,
                        status,
                        current_track,
                        Some(track),
                        pos == Some(i),
                        i == selected,
                        Style::default(),
                    );
                    items.push(ListItem::new(Spans::from(spans)));
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

fn flatten(
    spans: &mut Vec<Span>,
    xs: &Texts,
    status: &Status,
    current_track: Option<&Track>,
    queue_track: Option<&Track>,
    queue_current: bool,
    selected: bool,
    style: Style,
) {
    match xs {
        Texts::Text(x) => spans.push(Span::styled(x.clone(), style)),
        Texts::CurrentElapsed => {
            if let Some(Song { elapsed, .. }) = status.song {
                spans.push(Span::styled(
                    format!("{}:{:02}", elapsed / 60, elapsed % 60),
                    style,
                ));
            }
        }
        Texts::CurrentDuration => {
            if let Some(Track { time, .. }) = current_track {
                spans.push(Span::styled(
                    format!("{}:{:02}", time / 60, time % 60),
                    style,
                ));
            }
        }
        Texts::CurrentFile => {
            if let Some(Track { file, .. }) = current_track {
                spans.push(Span::styled(file.clone(), style));
            }
        }
        Texts::CurrentTitle => {
            if let Some(Track {
                title: Some(title), ..
            }) = current_track
            {
                spans.push(Span::styled(title.clone(), style));
            }
        }
        Texts::CurrentArtist => {
            if let Some(Track {
                artist: Some(artist),
                ..
            }) = current_track
            {
                spans.push(Span::styled(artist.clone(), style));
            }
        }
        Texts::CurrentAlbum => {
            if let Some(Track {
                album: Some(album), ..
            }) = current_track
            {
                spans.push(Span::styled(album.clone(), style));
            }
        }
        Texts::QueueDuration => {
            if let Some(Track { time, .. }) = queue_track {
                spans.push(Span::styled(
                    format!("{}:{:02}", time / 60, time % 60),
                    style,
                ));
            }
        }
        Texts::QueueFile => {
            if let Some(Track { file, .. }) = current_track {
                spans.push(Span::styled(file.clone(), style));
            }
        }
        Texts::QueueTitle => {
            if let Some(Track {
                title: Some(title), ..
            }) = queue_track
            {
                spans.push(Span::styled(title.clone(), style));
            }
        }
        Texts::QueueArtist => {
            if let Some(Track {
                artist: Some(artist),
                ..
            }) = queue_track
            {
                spans.push(Span::styled(artist.clone(), style));
            }
        }
        Texts::QueueAlbum => {
            if let Some(Track {
                album: Some(album), ..
            }) = queue_track
            {
                spans.push(Span::styled(album.clone(), style));
            }
        }
        Texts::Styled(styles, box xs) => {
            flatten(
                spans,
                xs,
                status,
                current_track,
                queue_track,
                queue_current,
                selected,
                patch_style(style, styles),
            );
        }
        Texts::Parts(xss) => {
            for xs in xss {
                flatten(
                    spans,
                    xs,
                    status,
                    current_track,
                    queue_track,
                    queue_current,
                    selected,
                    style,
                );
            }
        }
        Texts::If(cond, box yes, Some(box no)) => {
            flatten(
                spans,
                if eval_cond(cond, status, current_track, queue_current, selected) {
                    yes
                } else {
                    no
                },
                status,
                current_track,
                queue_track,
                queue_current,
                selected,
                style,
            );
        }
        Texts::If(cond, box xs, None) => {
            if eval_cond(cond, status, current_track, queue_current, selected) {
                flatten(
                    spans,
                    xs,
                    status,
                    current_track,
                    queue_track,
                    queue_current,
                    selected,
                    style,
                );
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

fn eval_cond(
    cond: &Condition,
    status: &Status,
    current_track: Option<&Track>,
    queue_current: bool,
    selected: bool,
) -> bool {
    match cond {
        Condition::Repeat => status.repeat,
        Condition::Random => status.random,
        Condition::Single => status.single == Some(true),
        Condition::Oneshot => status.single == None,
        Condition::Consume => status.consume,
        Condition::Playing => status.state == Some(true),
        Condition::Paused => status.state == Some(false),
        Condition::Stopped => status.state == None,
        Condition::TitleExist => matches!(current_track, Some(Track { title: Some(_), .. })),
        Condition::ArtistExist => matches!(
            current_track,
            Some(Track {
                artist: Some(_), ..
            }),
        ),
        Condition::AlbumExist => matches!(current_track, Some(Track { album: Some(_), .. })),
        Condition::QueueCurrent => queue_current,
        Condition::Selected => selected,
        Condition::Not(box x) => !eval_cond(x, status, current_track, queue_current, selected),
        Condition::And(box x, box y) => {
            eval_cond(x, status, current_track, queue_current, selected)
                && eval_cond(y, status, current_track, queue_current, selected)
        }
        Condition::Or(box x, box y) => {
            eval_cond(x, status, current_track, queue_current, selected)
                || eval_cond(y, status, current_track, queue_current, selected)
        }
        Condition::Xor(box x, box y) => {
            eval_cond(x, status, current_track, queue_current, selected)
                ^ eval_cond(y, status, current_track, queue_current, selected)
        }
    }
}
