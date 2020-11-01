use serde::{
    de::{self, EnumAccess, SeqAccess, VariantAccess, Visitor},
    Deserialize, Deserializer,
};
use tui::style::Color;

use std::fmt::{self, Formatter};

#[derive(Deserialize)]
pub struct Config {
    #[serde(default)]
    pub cycle: bool,
    #[serde(default = "jump_lines_default")]
    pub jump_lines: usize,
    #[serde(default = "ups_default")]
    pub ups: f64,
    pub layout: Widget,
}

fn jump_lines_default() -> usize {
    24
}

fn ups_default() -> f64 {
    4.0
}

#[derive(Deserialize)]
pub enum Widget {
    Rows(Vec<Constrained<Widget>>),
    Columns(Vec<Constrained<Widget>>),
    Textbox(Texts),
    Queue { columns: Vec<Constrained<Texts>> },
}

#[derive(Deserialize)]
pub enum Constrained<T> {
    Max(u16, T),
    Min(u16, T),
    Fixed(u16, T),
    Ratio(u32, T),
}

pub enum Texts {
    Text(String),
    CurrentElapsed,
    CurrentDuration,
    CurrentFile,
    CurrentTitle,
    CurrentArtist,
    CurrentAlbum,
    QueueDuration,
    QueueFile,
    QueueTitle,
    QueueArtist,
    QueueAlbum,
    Styled(Vec<AddStyle>, Box<Texts>),
    Parts(Vec<Texts>),
    If(Condition, Box<Texts>, Option<Box<Texts>>),
}

#[derive(Deserialize)]
pub enum AddStyle {
    Fg(Color),
    Bg(Color),
    Bold,
    NoBold,
    Dim,
    NoDim,
    Italic,
    NoItalic,
    Underlined,
    NoUnderlined,
    SlowBlink,
    NoSlowBlink,
    RapidBlink,
    NoRapidBlink,
    Reversed,
    NoReversed,
    Hidden,
    NoHidden,
    CrossedOut,
    NoCrossedOut,
}

#[derive(Deserialize)]
pub enum Condition {
    Playing,
    Repeat,
    Random,
    Single,
    Oneshot,
    Consume,
    TitleExist,
    ArtistExist,
    AlbumExist,
    Selected,
    Not(Box<Condition>),
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
    Xor(Box<Condition>, Box<Condition>),
}

impl<'de> Deserialize<'de> for Texts {
    fn deserialize<D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TextsVisitor;
        impl<'de> Visitor<'de> for TextsVisitor {
            type Value = Texts;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("enum Texts")
            }

            fn visit_enum<A: EnumAccess<'de>>(self, ea: A) -> Result<Self::Value, A::Error> {
                #[derive(Deserialize)]
                #[serde(field_identifier)]
                enum Variant {
                    Text,
                    CurrentElapsed,
                    CurrentDuration,
                    CurrentFile,
                    CurrentTitle,
                    CurrentArtist,
                    CurrentAlbum,
                    QueueDuration,
                    QueueFile,
                    QueueTitle,
                    QueueArtist,
                    QueueAlbum,
                    Styled,
                    Parts,
                    If,
                }

                struct StyledVisitor;
                impl<'de> Visitor<'de> for StyledVisitor {
                    type Value = Texts;

                    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                        formatter.write_str("variant Styled")
                    }

                    fn visit_seq<A: SeqAccess<'de>>(
                        self,
                        mut sa: A,
                    ) -> Result<Self::Value, A::Error> {
                        Ok(Texts::Styled(
                            sa.next_element()?
                                .ok_or_else(|| de::Error::invalid_length(0, &self))?,
                            sa.next_element()?
                                .ok_or_else(|| de::Error::invalid_length(1, &self))?,
                        ))
                    }
                }

                struct IfVisitor;
                impl<'de> Visitor<'de> for IfVisitor {
                    type Value = Texts;

                    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                        formatter.write_str("If variant")
                    }

                    fn visit_seq<A: SeqAccess<'de>>(
                        self,
                        mut sa: A,
                    ) -> Result<Self::Value, A::Error> {
                        Ok(Texts::If(
                            sa.next_element()?
                                .ok_or_else(|| de::Error::invalid_length(0, &self))?,
                            sa.next_element()?.map_or_else(
                                || Err(de::Error::invalid_length(1, &self)),
                                |x| Ok(Box::new(x)),
                            )?,
                            sa.next_element()?.map(Box::new),
                        ))
                    }
                }

                let (variant, va) = ea.variant()?;

                macro_rules! unit_variant {
                    ($v:ident) => {{
                        va.unit_variant()?;
                        Ok(Texts::$v)
                    }};
                }

                match variant {
                    Variant::Text => Ok(Texts::Text(va.newtype_variant()?)),
                    Variant::CurrentElapsed => unit_variant!(CurrentElapsed),
                    Variant::CurrentDuration => unit_variant!(CurrentDuration),
                    Variant::CurrentFile => unit_variant!(CurrentFile),
                    Variant::CurrentTitle => unit_variant!(CurrentTitle),
                    Variant::CurrentArtist => unit_variant!(CurrentArtist),
                    Variant::CurrentAlbum => unit_variant!(CurrentAlbum),
                    Variant::QueueDuration => unit_variant!(QueueDuration),
                    Variant::QueueFile => unit_variant!(QueueFile),
                    Variant::QueueTitle => unit_variant!(QueueTitle),
                    Variant::QueueArtist => unit_variant!(QueueArtist),
                    Variant::QueueAlbum => unit_variant!(QueueAlbum),
                    Variant::Styled => va.tuple_variant(2, StyledVisitor),
                    Variant::Parts => Ok(Texts::Parts(va.newtype_variant()?)),
                    Variant::If => va.tuple_variant(3, IfVisitor),
                }
            }
        }

        de.deserialize_enum(
            "Texts",
            &[
                "Text",
                "CurrentElapsed",
                "CurrentDuration",
                "CurrentFile",
                "CurrentTitle",
                "CurrentArtist",
                "CurrentAlbum",
                "QueueDuration",
                "QueueFile",
                "QueueTitle",
                "QueueArtist",
                "QueueAlbum",
                "Styled",
                "Parts",
                "If",
            ],
            TextsVisitor,
        )
    }
}
