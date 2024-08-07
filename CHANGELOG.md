# Changelog

## Unreleased

### Changed

## 0.10.2 - 2024-07-21

- Remove support for transferring via stdin and receiving to stdout
- Improve the code path taken when the `--mmap`-option is set

## 0.10.1 - 2024-07-15

### Fix

- [#37](https://github.com/CramBL/quick-file-transfer/issues/37) Root (`/`) not recognized as a valid path.

## 0.10.0 - 2024-07-14

### Added

- Configurable parallel jobs when running `qft evaluate-compression`
- Multi and single (configurable) Progress bar to `qft evaluate-compression`

## 0.9.0 - 2024-07-13

### Added

- Remote path validation for SSH (resolves [#7](https://github.com/CramBL/quick-file-transfer/issues/7))
- Send multiple simultaneous files in `qft ssh` mode.

### Changed

- `qft mdns discover` default timeout from `4s` -> `1s`
- Remove obsolete `qft send ssh` subcommand in favor of `qft ssh`.
- Remove obsolete `tcp-delay-ms` flag from `qft ssh`.
- Meaningful error message when using `qft ssh` targeting a remote that does not have `qft` in path.

### Fix

- #25 Excessive logging caused by tracing in the `ssh-rs`-crate
- #24 Support regular hostnames for scp-like mode
- #23 Support mDNS hostnames for scp-like mode
- Exponential backoff on TCP connection attempts only applying when the condition to stop trying is max connection attempts (as opposed to timeout).
- Remove 300ms sleep when initiating TCP connection after server is spawned over SSH (rely on polling with backoff instead).

## 0.8.0 - 2024-07-10

### Changed

- `qft mdns discover` default timeout from `5s` -> `4s`
- `qft mdns resolve` default timeout from `10s` -> `1s`
- Rework communication model for a multi-threaded approach. Now, for sending `N`-files, `N+1`-threads are spawned and `N+1`-sockets are used in total.

## 0.7.0 - 2024-06-30

### Changed

- Moving `qft send ssh` functionality to `qft ssh` and making it similar in syntax to `scp/rsync`.

### Fix

- Crash when sending multiple files over SSH with prealloc on

## 0.6.0 - 2024-06-29

### Added

- `get-free-port` command to retrieve a free port from `0.0.0.0` or a specified IP.
- `send ssh` command to send files to a remote host similar to how `scp` does it, except that the data transfer does not go over ssh, only the authentication happens over ssh.
- `--completions <SHELL>` generates shell completions for the specified shell, possible values: `bash`, `elvish`, `fish`, `powershell`, `zsh`.
- Allow specifying multiple instances of `--file <FILE>` to transfer more files at once. Has to be used with `--output-dir` on the server side.

### Fix

- Large files sent with `lz4` could end up missing a few tail end bytes.

### Changed

- Multiple changes to how command-line arguments are parsed (and which order/combination is valid). Note: This is not the last time this will be changed.

### Misc

- Update dependencies

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
