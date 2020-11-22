use tui::style::Color;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use crate::config::{
    AddStyle, Column, Condition, Config, Constrained, SearchFields, Texts, Widget,
};

pub fn config() -> Config {
    Config {
        address: address(),
        clear_query_on_play: false,
        cycle: false,
        jump_lines: jump_lines(),
        seek_secs: seek_secs(),
        search_fields: search_fields(),
        ups: ups(),
        layout: layout(),
    }
}

pub fn address() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6600)
}

pub fn jump_lines() -> usize {
    24
}

pub fn seek_secs() -> f32 {
    5.0
}

pub fn search_fields() -> SearchFields {
    SearchFields {
        file: false,
        title: true,
        artist: true,
        album: true,
    }
}

pub fn ups() -> f32 {
    1.0
}

pub fn layout() -> Widget {
    Widget::Rows(vec![
        Constrained::Fixed(
            1,
            Widget::Columns(vec![
                Constrained::Ratio(
                    12,
                    Widget::Textbox(Texts::Styled(
                        vec![AddStyle::Fg(Color::Indexed(122)), AddStyle::Bold],
                        box Texts::Text(String::from("Title")),
                    )),
                ),
                Constrained::Ratio(
                    10,
                    Widget::Textbox(Texts::Styled(
                        vec![AddStyle::Fg(Color::Indexed(158)), AddStyle::Bold],
                        box Texts::Text(String::from("Artist")),
                    )),
                ),
                Constrained::Ratio(
                    10,
                    Widget::Textbox(Texts::Styled(
                        vec![AddStyle::Fg(Color::Indexed(194)), AddStyle::Bold],
                        box Texts::Text(String::from("Album")),
                    )),
                ),
                Constrained::Ratio(
                    1,
                    Widget::Textbox(Texts::Styled(
                        vec![AddStyle::Fg(Color::Indexed(230)), AddStyle::Bold],
                        box Texts::Text(String::from("Time")),
                    )),
                ),
            ]),
        ),
        Constrained::Min(
            0,
            Widget::Queue(vec![
                Column {
                    item: Constrained::Ratio(
                        12,
                        Texts::If(
                            Condition::QueueCurrent,
                            box Texts::Styled(vec![AddStyle::Italic], box Texts::QueueTitle),
                            Some(box Texts::QueueTitle),
                        ),
                    ),
                    style: vec![AddStyle::Fg(Color::Indexed(75))],
                    selected_style: vec![
                        AddStyle::Fg(Color::Black),
                        AddStyle::Bg(Color::Indexed(75)),
                        AddStyle::Bold,
                    ],
                },
                Column {
                    item: Constrained::Ratio(
                        10,
                        Texts::If(
                            Condition::QueueCurrent,
                            box Texts::Styled(vec![AddStyle::Italic], box Texts::QueueArtist),
                            Some(box Texts::QueueArtist),
                        ),
                    ),
                    style: vec![AddStyle::Fg(Color::Indexed(111))],
                    selected_style: vec![
                        AddStyle::Fg(Color::Black),
                        AddStyle::Bg(Color::Indexed(111)),
                        AddStyle::Bold,
                    ],
                },
                Column {
                    item: Constrained::Ratio(
                        10,
                        Texts::If(
                            Condition::QueueCurrent,
                            box Texts::Styled(vec![AddStyle::Italic], box Texts::QueueAlbum),
                            Some(box Texts::QueueAlbum),
                        ),
                    ),
                    style: vec![AddStyle::Fg(Color::Indexed(147))],
                    selected_style: vec![
                        AddStyle::Fg(Color::Black),
                        AddStyle::Bg(Color::Indexed(147)),
                        AddStyle::Bold,
                    ],
                },
                Column {
                    item: Constrained::Ratio(
                        1,
                        Texts::If(
                            Condition::QueueCurrent,
                            box Texts::Styled(vec![AddStyle::Italic], box Texts::QueueDuration),
                            Some(box Texts::QueueDuration),
                        ),
                    ),
                    style: vec![AddStyle::Fg(Color::Indexed(183))],
                    selected_style: vec![
                        AddStyle::Fg(Color::Black),
                        AddStyle::Bg(Color::Indexed(183)),
                        AddStyle::Bold,
                    ],
                },
            ]),
        ),
        Constrained::Fixed(
            1,
            Widget::Columns(vec![
                Constrained::Min(
                    0,
                    Widget::Textbox(Texts::Styled(
                        vec![AddStyle::Bold],
                        box Texts::If(
                            Condition::Searching,
                            box Texts::Parts(vec![
                                Texts::Styled(
                                    vec![AddStyle::Fg(Color::Indexed(113))],
                                    box Texts::Text(String::from("Searching: ")),
                                ),
                                Texts::Styled(
                                    vec![AddStyle::Fg(Color::Indexed(185))],
                                    box Texts::Query,
                                ),
                            ]),
                            Some(box Texts::If(
                                Condition::Not(box Condition::Stopped),
                                box Texts::Parts(vec![
                                    Texts::Styled(
                                        vec![AddStyle::Fg(Color::Indexed(113))],
                                        box Texts::Parts(vec![
                                            Texts::If(
                                                Condition::Playing,
                                                box Texts::Text(String::from("[playing: ")),
                                                Some(box Texts::Text(String::from("[paused:  "))),
                                            ),
                                            Texts::CurrentElapsed,
                                            Texts::Text(String::from("/")),
                                            Texts::CurrentDuration,
                                            Texts::Text(String::from("] ")),
                                        ]),
                                    ),
                                    Texts::If(
                                        Condition::TitleExist,
                                        box Texts::Parts(vec![
                                            Texts::Styled(
                                                vec![AddStyle::Fg(Color::Indexed(149))],
                                                box Texts::CurrentTitle,
                                            ),
                                            Texts::If(
                                                Condition::ArtistExist,
                                                box Texts::Parts(vec![
                                                    Texts::Styled(
                                                        vec![AddStyle::Fg(Color::Indexed(216))],
                                                        box Texts::Text(String::from(" ◆ ")),
                                                    ),
                                                    Texts::Styled(
                                                        vec![AddStyle::Fg(Color::Indexed(185))],
                                                        box Texts::CurrentArtist,
                                                    ),
                                                    Texts::If(
                                                        Condition::AlbumExist,
                                                        box Texts::Parts(vec![
                                                            Texts::Styled(
                                                                vec![AddStyle::Fg(Color::Indexed(
                                                                    216,
                                                                ))],
                                                                box Texts::Text(String::from(
                                                                    " ◆ ",
                                                                )),
                                                            ),
                                                            Texts::Styled(
                                                                vec![AddStyle::Fg(Color::Indexed(
                                                                    221,
                                                                ))],
                                                                box Texts::CurrentAlbum,
                                                            ),
                                                        ]),
                                                        None,
                                                    ),
                                                ]),
                                                None,
                                            ),
                                        ]),
                                        Some(box Texts::Styled(
                                            vec![AddStyle::Fg(Color::Indexed(185))],
                                            box Texts::CurrentFile,
                                        )),
                                    ),
                                ]),
                                None,
                            )),
                        ),
                    )),
                ),
                Constrained::Fixed(
                    7,
                    Widget::TextboxR(Texts::Styled(
                        vec![AddStyle::Fg(Color::Indexed(81))],
                        box Texts::Parts(vec![
                            Texts::Text(String::from("[")),
                            Texts::If(Condition::Repeat, box Texts::Text(String::from("@")), None),
                            Texts::If(Condition::Random, box Texts::Text(String::from("#")), None),
                            Texts::If(
                                Condition::Single,
                                box Texts::Text(String::from("^")),
                                Some(box Texts::If(
                                    Condition::Oneshot,
                                    box Texts::Text(String::from("!")),
                                    None,
                                )),
                            ),
                            Texts::If(Condition::Consume, box Texts::Text(String::from("*")), None),
                            Texts::Text(String::from("]")),
                        ]),
                    )),
                ),
            ]),
        ),
    ])
}
