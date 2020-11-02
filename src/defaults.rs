use tui::style::Color;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use crate::config::{AddStyle, Column, Condition, Config, Constrained, Texts, Widget};

pub fn config() -> Config {
    Config {
        address: address(),
        cycle: false,
        jump_lines: jump_lines(),
        seek_secs: seek_secs(),
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

pub fn seek_secs() -> f64 {
    5.0
}

pub fn ups() -> f64 {
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
                        Box::new(Texts::Text(String::from("Title"))),
                    )),
                ),
                Constrained::Ratio(
                    10,
                    Widget::Textbox(Texts::Styled(
                        vec![AddStyle::Fg(Color::Indexed(158)), AddStyle::Bold],
                        Box::new(Texts::Text(String::from("Artist"))),
                    )),
                ),
                Constrained::Ratio(
                    10,
                    Widget::Textbox(Texts::Styled(
                        vec![AddStyle::Fg(Color::Indexed(194)), AddStyle::Bold],
                        Box::new(Texts::Text(String::from("Album"))),
                    )),
                ),
                Constrained::Ratio(
                    1,
                    Widget::Textbox(Texts::Styled(
                        vec![AddStyle::Fg(Color::Indexed(230)), AddStyle::Bold],
                        Box::new(Texts::Text(String::from("Time"))),
                    )),
                ),
            ]),
        ),
        Constrained::Min(
            0,
            Widget::Queue(vec![
                Column {
                    item: Constrained::Ratio(12, Texts::QueueTitle),
                    style: vec![AddStyle::Fg(Color::Indexed(75))],
                    selected_style: vec![
                        AddStyle::Fg(Color::Black),
                        AddStyle::Bg(Color::Indexed(75)),
                        AddStyle::Bold,
                    ],
                },
                Column {
                    item: Constrained::Ratio(10, Texts::QueueArtist),
                    style: vec![AddStyle::Fg(Color::Indexed(111))],
                    selected_style: vec![
                        AddStyle::Fg(Color::Black),
                        AddStyle::Bg(Color::Indexed(111)),
                        AddStyle::Bold,
                    ],
                },
                Column {
                    item: Constrained::Ratio(10, Texts::QueueAlbum),
                    style: vec![AddStyle::Fg(Color::Indexed(147))],
                    selected_style: vec![
                        AddStyle::Fg(Color::Black),
                        AddStyle::Bg(Color::Indexed(147)),
                        AddStyle::Bold,
                    ],
                },
                Column {
                    item: Constrained::Ratio(1, Texts::QueueDuration),
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
                    Widget::Textbox(Texts::If(
                        Condition::Not(Box::new(Condition::Stopped)),
                        Box::new(Texts::Styled(
                            vec![AddStyle::Bold],
                            Box::new(Texts::Parts(vec![
                                Texts::Styled(
                                    vec![AddStyle::Fg(Color::Indexed(113))],
                                    Box::new(Texts::Parts(vec![
                                        Texts::If(
                                            Condition::Playing,
                                            Box::new(Texts::Text(String::from("[playing: "))),
                                            Some(Box::new(Texts::Text(String::from("[paused:  ")))),
                                        ),
                                        Texts::CurrentElapsed,
                                        Texts::Text(String::from("/")),
                                        Texts::CurrentDuration,
                                        Texts::Text(String::from("] ")),
                                    ])),
                                ),
                                Texts::If(
                                    Condition::TitleExist,
                                    Box::new(Texts::Parts(vec![
                                        Texts::Styled(
                                            vec![AddStyle::Fg(Color::Indexed(149))],
                                            Box::new(Texts::CurrentTitle),
                                        ),
                                        Texts::If(
                                            Condition::ArtistExist,
                                            Box::new(Texts::Parts(vec![
                                                Texts::Styled(
                                                    vec![AddStyle::Fg(Color::Indexed(216))],
                                                    Box::new(Texts::Text(String::from(" ◆ "))),
                                                ),
                                                Texts::Styled(
                                                    vec![AddStyle::Fg(Color::Indexed(185))],
                                                    Box::new(Texts::CurrentArtist),
                                                ),
                                                Texts::If(
                                                    Condition::AlbumExist,
                                                    Box::new(Texts::Parts(vec![
                                                        Texts::Styled(
                                                            vec![AddStyle::Fg(Color::Indexed(216))],
                                                            Box::new(Texts::Text(String::from(
                                                                " ◆ ",
                                                            ))),
                                                        ),
                                                        Texts::Styled(
                                                            vec![AddStyle::Fg(Color::Indexed(221))],
                                                            Box::new(Texts::CurrentAlbum),
                                                        ),
                                                    ])),
                                                    None,
                                                ),
                                            ])),
                                            None,
                                        ),
                                    ])),
                                    Some(Box::new(Texts::Styled(
                                        vec![AddStyle::Fg(Color::Indexed(185))],
                                        Box::new(Texts::CurrentFile),
                                    ))),
                                ),
                            ])),
                        )),
                        None,
                    )),
                ),
                Constrained::Fixed(
                    12,
                    Widget::TextboxR(Texts::Styled(
                        vec![AddStyle::Fg(Color::Indexed(81))],
                        Box::new(Texts::Parts(vec![
                            Texts::Text(String::from("[")),
                            Texts::If(
                                Condition::Repeat,
                                Box::new(Texts::Text(String::from("@"))),
                                None,
                            ),
                            Texts::If(
                                Condition::Random,
                                Box::new(Texts::Text(String::from("#"))),
                                None,
                            ),
                            Texts::If(
                                Condition::Single,
                                Box::new(Texts::Text(String::from("^"))),
                                Some(Box::new(Texts::If(
                                    Condition::Oneshot,
                                    Box::new(Texts::Text(String::from("!"))),
                                    None,
                                ))),
                            ),
                            Texts::If(
                                Condition::Consume,
                                Box::new(Texts::Text(String::from("*"))),
                                None,
                            ),
                            Texts::Text(String::from("]")),
                        ])),
                    )),
                ),
            ]),
        ),
    ])
}
