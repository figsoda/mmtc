# mmtc

[![release](https://img.shields.io/github/v/release/figsoda/mmtc?logo=github&style=flat-square)](https://github.com/figsoda/mmtc/releases)
[![version](https://img.shields.io/crates/v/mmtc?logo=rust&style=flat-square)][Crate]
[![dependencies](https://img.shields.io/librariesio/release/cargo/mmtc?style=flat-square)](https://libraries.io/cargo/mmtc)
[![license](https://img.shields.io/badge/license-MPL--2.0-blue?style=flat-square)](https://www.mozilla.org/en-US/MPL/2.0)
[![ci](https://img.shields.io/github/workflow/status/figsoda/mmtc/ci?label=ci&logo=github-actions&style=flat-square)](https://github.com/figsoda/mmtc/actions?query=workflow:ci)

Minimal [mpd](https://github.com/musicplayerdaemon/mpd) terminal client that aims to be simple yet highly configurable

- [Installation](#installation)
- [Building from source](#building-from-source)
- [Usage](#usage)
- [Environment variables](#environment-variables)
- [Key bindings](#key-bindings)
- [Configuration.md]
- [CHANGELOG.md]


## Installation

The latest precompiled binaries are available on [github](https://github.com/figsoda/mmtc/releases/latest).

Alternatively you can install mmtc from [crates.io][Crate] with cargo.

```sh
cargo install mmtc
```


## Building from source

```sh
cargo build --release
```


## Usage

```sh
mmtc [FLAGS] [OPTIONS]
```

flag | description
-|-
--clear-query-on-play | Clear query on play
--cycle | Cycle through the queue
-h, --help | Prints help information
--no-clear-query-on-play | Don't clear query on play
--no-cycle | Don't cycle through the queue
-V, --version | Prints version information

option | description
-|-
--address \<address> | Specify the address of the mpd server
-c, --config \<file> | Specify the config file
--jump-lines \<number> | The number of lines to jump
--seek-secs \<number> | The time to seek in seconds
--ups \<number> | The amount of status updates per second


## Environment variables

Setting both `MPD_HOST` and `MPD_PORT` is the equalvalent of `--address $MPD_HOST:$MPD_PORT`

Precedence: command line arguments > environment variables > configuration file


## Key bindings

Key | Action
-|-
<kbd>q</kbd> or <kbd>Ctrl</kbd> + <kbd>q</kbd> | quit mmtc
<kbd>r</kbd> | toggle repeat
<kbd>R</kbd> | toggle random
<kbd>s</kbd> | toggle single
<kbd>S</kbd> | toggle oneshot
<kbd>c</kbd> | toggle consume
<kbd>p</kbd> | toggle pause
<kbd>;</kbd> | stop
<kbd>h</kbd> or <kbd>Left</kbd> | seek backwards
<kbd>l</kbd> or <kbd>Right</kbd> | seek forwards
<kbd>H</kbd> | previous song
<kbd>L</kbd> | next song
<kbd>Enter</kbd> | play selected song or quit searching mode if in searching mode
<kbd>Space</kbd> | select current song or the first song in the queue
<kbd>j</kbd>, <kbd>Down</kbd> or <kbd>ScrollDown</kbd> | go down in the queue
<kbd>k</kbd>, <kbd>Up</kbd> or <kbd>ScrollUp</kbd> | go up in the queue
<kbd>J</kbd> or <kbd>PageDown</kbd> | jump down in the queue
<kbd>K</kbd> or <kbd>PageUp</kbd> | jump up in the queue
<kbd>g</kbd> | go to the top of the queue
<kbd>G</kbd> | go to the bottom of the queue
<kbd>/</kbd> | enter searching mode
<kbd>Escape</kbd> | quit searching mode and empty query


## Configuration

See [Configuration.md]


## Changelog

See [CHANGELOG.md]


[Crate]: https://crates.io/crates/mmtc
[Configuration.md]: Configuration.md
[CHANGELOG.md]: CHANGELOG.md
