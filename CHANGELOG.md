# Changelog

## [Unreleased]

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
