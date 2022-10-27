# Changelog

## v0.3.0 - 2022-10-27

### Features
- Add `--cmd` flag for running arbitrary mpd commands
- Man page


## v0.2.15 - 2022-09-29

### Internal
- Switch to Rust 2021 edition
- Update dependencies


## v0.2.14 - 2021-11-08

### Features
- Add cursor to search bar in default settings
- New binding - <kbd>Ctrl</kbd> + <kbd>u</kbd> to clear search
- New binding - <kbd>Ctrl</kbd> + <kbd>d</kbd> to jump down in the queue
- New binding - <kbd>Ctrl</kbd> + <kbd>u</kbd> to jump up in the queue


## v0.2.13 - 2021-02-19

### Features
- Completions for bash, elvish, fish, powershell, and zsh
- Support hostname resolution for command line argument `--address`


## v0.2.12 - 2021-02-07

### Features
- Support for hostname resolution ([#3](https://github.com/figsoda/mmtc/issues/3))


## v0.2.11 - 2021-02-07

### Features
- Support for MPD_HOST and MPD_PORT environment variables ([#2](https://github.com/figsoda/mmtc/issues/2))


## v0.2.10 - 2021-02-05

### Fixes
- Now correctly toggles pause


## v0.2.9 - 2021-02-05

### Changes
- Allow multiple occurrences of command line flags

### Compatibility
- Migrate to stable rust


## v0.2.8 - 2021-01-16

### Fixes
- Fixed scrolling direction with mouse wheels


## v0.2.7 - 2021-01-15

### Features
- New binding - <kbd>g</kbd> to go to the top of the queue
- New binding - <kbd>G</kbd> to go to the bottom of the queue


## v0.2.6 - 2020-11-29

### Fixes
- Fixed jumping up in the queue logic when cycle option is turned on


## v0.2.5 - 2020-11-20

### Fixes
- Fixed hang in extreme conditions


## v0.2.4 - 2020-11-17

### Features
- Allow seeking backwards with left key and forwards with right key when searching
- New binding - <kbd>Ctrl</kbd> + <kbd>q</kbd> to quit mmtc

### Optimization
- Various performance improvements


## v0.2.3 - 2020-11-13

### Changes
- On toggle pause, play a song if none is playing
- Reduce default size of player state textbox from 12 to 7

### Fixes
- Fixed delay after quitting search with an empty query


## v0.2.2 - 2020-11-09

### Changes
- The current song would now be selected after quitting searching mode or emptying query

### Features
- Allow navigating with down, up, page down and page up keys when searching
- New command line flags - `--clear-query-on-play` and `--no-clear-query-on-play`
- New config - `clear_query_on_play`


## v0.2.1 - 2020-11-07

### Features
- Allow scrolling with mouse when searching

### Fixes
- Correctly handle resizing when searching


## v0.2.0 - 2020-11-05

### Changes
- Replace `--cycle` option with `--cycle` and `--no-cycle` flags

### Features
- Search tracks in your queue via `/`
- New config - `search_fields`
- New `Condition`s - `Searching` and `Filtered`
- New `Texts` - `Query`


## v0.1.1 - 2020-11-03

### Changes
- No longer adds extra zeros before the minute number

### Documentation
- Added [configuration instructions](Configuration.md)

### Features
- New `Condition` - `QueueCurrent`


## v0.1.0 - 2020-11-02

### Features
- Minimal mpd terminal client
