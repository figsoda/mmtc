# Changelog


## v0.2.2 - 2020-11-08

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
- Added [Configuration instructions](https://github.com/figsoda/mmtc/blob/main/Configuration.md)

### Features
- New `Condition` - `QueueCurrent`


## v0.1.0 - 2020-11-02

### Features
- Minimal mpd terminal client
