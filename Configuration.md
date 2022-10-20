# Configuration


## File Resolution

By default, mmtc looks for configuration file at `<your config directory>/mmtc/mmtc.ron`.
Your config directory may differ depending on the operating system, so mmtc tries to find your config directory with [dirs crate](https://docs.rs/dirs-next/*/dirs_next/fn.config_dir.html).

This setting can be overwritten by the command line argument `-c` or `--config`.

If no config file was given from the command line and mmtc failed to find your config directory, the [default configuration](mmtc.ron) would be used.


## File Structure

The configuration file is written in [ron](https://github.com/ron-rs/ron), an expressive object notation.
Check out its [specification wiki](https://github.com/ron-rs/ron/wiki/Specification) if you are having trouble figuring out its syntax.
The whole file should be a [`Config` struct](#Config).

### Config

Type: struct

field | type | description | default
-|-|-|-
`address` | string | the address of the mpd server | `"127.0.0.1:6600"`
`clear_query_on_play` | boolean | clear query on play | `false`
`cycle` | boolean |  cycle through the queue | `false`
`jump_lines` | non-negative integer | the number of lines to jump | `24`
`seek_secs` | non-negative number | the time to seek in seconds | `5.0`
`search_fields` | [`SearchFields`](#SearchFields) | the fields to index from when searching | see [`SearchFields`](#SearchFields)
`ups` | non-negative number | the amount of status updates per second | `1.0`
`layout` | [`Widget`](#Widget) | the layout of the application | see [mmtc.ron](mmtc.ron)

### SearchFields

Type: struct

field | type | description | default
-|-|-|-
`file` | boolean | whether to search in file names | `false`
`title` | boolean | whether to search in titles | `true`
`artist` | boolean | whether to search in artists | `true`
`album` | boolean | whether to search in albums | `true`

### Widget

Type: enum

variant | struct, tuple or unit | fields | description
-|-|-|-
`Rows(rows)` | tuple | list of [`Constrained`](#Constrained) [`Widget`s](#Widget) | split into rows
`Columns(columns)` | tuple | list of [`Constrained`](#Constrained) [`Widget`s](#Widget) | split into columns
`Textbox(texts)` or `TextboxL(texts)` | tuple | [`Texts`](#Texts) | text with left alignment
`TextboxC(texts)` | tuple | [`Texts`](#Texts) | text with center alignment
`TextboxR(texts)` | tuple | [`Texts`](#Texts) | text with right alignment
`Queue(columns)` | tuple | list of [`Column`](#Column) | displays the queue

### Constrained

Type: enum

variant | struct, tuple or unit | fields (separated by comma) | description
-|-|-|-
`Max(n, item)` | tuple | non-negative integer, \<type> | `item` with a maximum length of `n`
`Min(n, item)` | tuple | non-negative integer, \<type> | `item` with a minimum length of `n`
`Fixed(n, item)` | tuple | non-negative integer, \<type> | `item` with a fixed length of `n`
`Ratio(n, item)` | tuple | non-negative integer, \<type> | divide the total length in to ratios, mixing with other constraints would cut off the rightmost item

### Texts

Type: enum

variant | struct, tuple or unit | fields (separated by comma) | description
-|-|-|-
`Text(str)` | tuple | string | plain text
`CurrentElapsed` | unit | | time elapsed of the current song
`CurrentDuration` | unit | | total duration of the current song
`CurrentFile` | unit | | file name of the current song
`CurrentArtist` | unit | | artist of the current song
`QueueAlbum` | unit | | album of the song in queue (only works inside a [`Queue` `Widget`](#Widget))
`QueueDuration` | unit | | total duration of the song in queue (only works inside a `Queue` [`Widget`](#Widget))
`QueueFile` | unit | | file name of the song in queue (only works inside a [`Queue` `Widget`](#Widget))
`QueueArtist` | unit | | artist of the song in queue (only works inside a [`Queue` `Widget`](#Widget))
`QueueAlbum` | unit | | album of the song in queue (only works inside a [`Queue` `Widget`](#Widget))
`Query` | unit | | current query
`Styled(styles, texts)` | tuple | list of [`Style`](#Style), [`Texts`](#Texts) | styled text
`Parts(parts)` | tuple | list of [`Texts`](#Texts) | concatenate multiple parts of texts
`If(condition, lhs, rhs)` or `If(condition, lhs)` | tuple | [`Condition`](#Condition), [`Texts`](#Texts), optional [`Texts`](#Texts) | if `condition` then `lhs` (else `rhs`)

### Style

Type: enum

Note: some styles may not work depending on your terminal emulator

variant | struct, tuple or unit | fields | description
-|-|-|-
`Fg(color)` | tuple | [`Color`](#Color) | change foreground color
`Bg(color)` | tuple | [`Color`](#Color) | change background color
`Bold` | unit | | bold
`NoBold` | unit | | remove bold
`Dim` | unit | | dim
`NoDim` | unit | | remove dim
`Italic` | unit | | italic
`NoItalic` | unit | | remove italic
`Underlined` | unit | | underlined
`NoUnderlined` | unit | | remove underlined
`SlowBlink` | unit | | slow blink
`NoSlowBlink` | unit | | remove slow blink
`RapidBlink` | unit | | rapid blink
`NoRapidBlink` | unit | | remove slow blink
`Reversed` | unit | | reversed
`NoReversed` | unit | | remove reversed
`Hidden` | unit | | hidden
`NoHidden` | unit | | remove hidden
`CrossedOut` | unit | | crossed out
`NoCrossedOut` | unit | | remove crossed out

### Color

Type: enum

variant | struct, tuple or unit | fields (separated by comma) | description
-|-|-|-
`Reset` | unit | | reset to default color
`Black` | unit | | black
`Red` | unit | | red
`Green` | unit | | green
`Yellow` | unit | | tellow
`Blue` | unit | | blue
`Magenta` | unit | | magenta
`Cyan` | unit | | cyan
`Gray` | unit | | gray
`DarkGray` | unit | | dark gray
`LightRed` | unit | | light red
`LightGreen` | unit | | light green
`LightYellow` | unit | | light yellow
`LightBlue` | unit | | light blue
`LightMagenta` | unit | | light magenta
`LightCyan` | unit | | light cyan
`White` | unit | | white
`Rgb(r, g, b)` | tuple | 0 to 255, 0 to 255, 0 to 255 | rgb color
`Indexed(n)` | tuple | 0 to 255 | the `n`th color of 256 preset colors

### Condition

Type: enum, evaluates to a boolean

variant | struct, tuple or unit | fields (separated by comma) | description
-|-|-|-
`Repeat` | unit | | whether mpd is in repeat mode
`Random` | unit | | whether mpd is in random mode
`Single` | unit | | whether mpd is in single mode
`Oneshot` | unit | | whether mpd is in oneshot mode
`Consume` | unit | | whether mpd is in consume mode
`Playing` | unit | | whether the song is playing
`Paused` | unit | | whether the song is paused
`Stopped` | unit | | whether there is no song playing or paused
`TitleExist` | unit | | whether the current song has a title
`ArtistExist` | unit | | whether the current song has an artist
`QueueCurrent` | unit | | whether the song in queue is the current song (only works inside a `Queue` [`Widget`](#Widget))
`Selected` | unit | | whether the song in queue is selected (only works inside a `Queue` [`Widget`](#Widget))
`Searching` | unit | | whether mmtc is in searching mode
`Filtered` | unit | | whether the queue is filtered by a query
`Not(condition)` | tuple | [`Condition`](#Condition) | logical not
`And(lhs, rhs)` | tuple | [`Condition`](#Condition), [`Condition`](#Condition) | logical and
`Or(lhs, rhs)` | tuple | [`Condition`](#Condition), [`Condition`](#Condition) | logical or
`Xor(lhs, rhs)` | tuple | [`Condition`](#Condition), [`Condition`](#Condition) | logical exclusive or

### Column

Type: struct

field | type | description | default
-|-|-|-
`item` | [`Constrained`](#Constrained) [`Texts`](#Texts) | `Queue` [`Widget`](#Widget) creates an `item` for each track in your queue for each column | mandatory, no default value
`style` | list of [`Style`s](Style) | style of the item when not selected | `[]`
`selected_style` | list of [`Style`s](Style) | style of the item when selected | `[]`
