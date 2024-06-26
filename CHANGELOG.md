# Changelog

## [Unreleased]

### Added

- `get-free-port` command to retrieve a free port from `0.0.0.0` or a specified IP.
- `send ssh` command to send files to a remote host similar to how `scp` does it, except that the data transfer does not go over ssh, only the authentication happens over ssh.

### Fix

- Large files sent with `lz4` could end up missing a few tail end bytes.

### Changed

- Multiple changes to how command-line arguments are parsed (and which order/combination is valid). Note: This is not the last time this will be changed.

## 0.5.0 - 2024-06-21

### Added

- Pretty table formatted `evaluate-compression` output

### Changed

- Features flags `mdns` & `evaluate-compression` to allow for opting out of those features (and their dependencies) they are enabled by default.

## 0.4.0 - 2024-06-20

### Added

- `qft dns resolve` --short-circuit flag
- Configurable compression levels for `bzip2`, `gzip`, and `xz`, also used in `compression-evaluation`.
- `--color=<WHEN>`  [default: auto] [possible values: auto, always, never] to toggle colors in prints to stderr/stdout.
- `--omit-levels [<OMIT_LEVELS>...]` List of compression levels to omit from evaluation

## 0.3.0 - 2024-06-19

### Added

- `qft evaluate-compression` allows passing a file which `qft` will evaluate each compression type on an output the results.
- `xz` support
- `bzip2` support

## 0.2.0 - 2024-06-18

### Added

- Allow partial `.local` or fully `.local.` qualified hostnames in `qft mdns resolve`

### Changed

- `qft mdns resolve` now accepts the hostname as the first argument instead of requiring it passed via `--hostname`

## 0.1.0 - 2024-06-18

### Added

- `qft listen` start a server in listen mode with various options for how to receive some content.
- `qft send [ip|mdns]` start a client that sends to a server addressed by hostname or ip with various send options such as `--prealloc` and/or `--compression [lz4|gzip|none]`
- `qft mdns` utility with subcommands `discover/resolve/register`
