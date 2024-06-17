# Quick File Transfer (qft)

- [Quick File Transfer (qft)](#quick-file-transfer-qft)
  - [Purpose](#purpose)
  - [Usage](#usage)
  - [Example (outdated)](#example-outdated)
    - [Host #1](#host-1)
    - [Host #2](#host-2)
  - [Example CI script (outdated)](#example-ci-script-outdated)
  - [Supported compression formats](#supported-compression-formats)

## Purpose

Transfer files as **quickly**, **safely**, and **painlessly** as possible on a local network.

`qft` optimizes for a scenario where embedded systems regularly transfer large files across a local network, such as a continuous integration pipeline where firmware (e.g. Rauc) can take significant time to transfer with tools such as `rsync`, `scp`, or `netcat`.

To accomplish this, `qft` acts as a server/client that transfers data over TCP. It is very similar to how `netcat` can be used to transfer files, but `qft` focuses solely on transferring files, and comes with a variety of customization options such as [compression/decompression](#supported-compression-formats), memory mapping, preallocation options and more. TCP is chosen for reliable data transfer, and no authentication or encryption is layered on top to reduce the overhead.

If you are worried about a man-in-the-middle, you can simply check your data on the receiving end before continuing. There should be no additional security concerns (if you disagree, please create an issue highlighting the concern).

## Usage

```markdown
$ qft -h
Usage: qft [OPTIONS] <COMMAND>

Commands:
  listen  Run in listen (server) mode
  send    Run in Connect (client) mode
  mdns    Use mDNS utilities
  help    Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose...                 Pass many times for more log output
  -q, --quiet                      Silence all output [env: QFT_QUIET=]
  -m, --message <MESSAGE>          Send a message to the server
  -f, --file <FILE>                Supply a file for I/O (if none: use stdio)
  -c, --compression <COMPRESSION>  Compression format [possible values: gzip, bzip2, xz, lz4, none]
      --mmap                       Use memory mapping mode
      --prealloc                   Client will send the size of the file to the server allowing the server to preallocate for the expected size
  -h, --help                       Print help (see more with '--help')
  -V, --version                    Print version
```

## Example (outdated)

In a CI script Host #2 could simply ssh into Host #1 and launch the `qft listen` command as a background process before invoking `qft connect`.

### Host #1

Listen on port `1234`.

```shell
qft --ip 0.0.0.0 --port 1234 --file received.data listen
```

### Host #2

Transfer a file to **Host #1**.

```shell
qft --ip <HOST-1-IP> --port 1234 --file transfer.data connect
```

## Example CI script (outdated)

Something like a Raspberry Pi could orchestrate the testing of an embedded system, and might use a script like this to transfer a firmware upgrade bundle.

```bash
#!/usr/bin/env bash
set -eu
HOST1_IP="x.x.x.x"
PORT=1234
FIRMWARE="fw.raucb"
ssh -f user@${HOST1_IP} "sh -c 'nohup qft --ip ${HOST1_IP} --port ${PORT} --file ${FIRMWARE} listen > qft_listen.log 2>&1 &'"
qft --ip ${HOST1_IP} --port ${PORT} --file ${FIRMWARE} connect
ssh user@${HOST1_IP} -t "rauc install ${FIRMWARE}"
...
```

## Supported compression formats

- [ ] bz2
- [x] gzip
- [x] lz4
- [ ] xz
