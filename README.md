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
-a, --address <address>          Specify the address of the mpd server
-c, --config <config>            Specify the config file
-c, --cycle <cycle>              Cycle through the queue
-j, --jump-lines <jump-lines>    The number of lines to jump
-s, --seek-secs <seek-secs>      The time in seconds to seek
-u, --ups <ups>                  The amount of status updates per second
```


## Changelog

See [CHANGELOG.md](https://github.com/figsoda/mmtc/blob/main/CHANGELOG.md)
