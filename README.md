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


## Changelog

See [CHANGELOG.md](https://github.com/figsoda/mmtc/blob/main/CHANGELOG.md)
