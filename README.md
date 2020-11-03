# mmtc

Minimal mpd terminal client


## Building from source

This requires the nightly toolchain of Rust.

```shell
cargo +nightly build --release
```


## Usage

```
mmtc [OPTIONS]
```

### FLAGS

```
-h, --help       Prints help information
-V, --version    Prints version information
```

### OPTIONS

```
    --address <address>          Specify the address of the mpd server
-c, --config <config>            Specify the config file
    --cycle <cycle>              Cycle through the queue
    --jump-lines <jump-lines>    The number of lines to jump
    --seek-secs <seek-secs>      The time in seconds to seek
    --ups <ups>                  The amount of status updates per second
```


## Key bindings


Key | Action
-|-
`q` | quit mmtc
`r` | toggle repeat
`R` | toggle random
`s` | toggle single
`S` | toggle oneshot
`c` | toggle consume
`p` | toggle pause
`;` | stop
`h`, `Left` | seek backwards
`l`, `Right` | seek forwards
`H` | previous song
`L` | next song
`Enter` | play selected song
`Space` | select current song or the first song in the queue
`j`, `Down`, `ScrollDown` | go down in the queue
`k`, `Up`, `ScrollUp` | go up in the queue
`J`, `PageDown` | jump down in the queue
`K`, `PageUp` | jump up in the queue


## Changelog

See [CHANGELOG.md](https://github.com/figsoda/mmtc/blob/main/CHANGELOG.md)
