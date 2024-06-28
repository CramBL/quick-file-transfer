# Quick File Transfer (qft)

[![CI](https://github.com/CramBL/quick-file-transfer/actions/workflows/CI.yml/badge.svg)](https://github.com/CramBL/quick-file-transfer/actions/workflows/CI.yml)

- [Quick File Transfer (qft)](#quick-file-transfer-qft)
  - [Purpose](#purpose)
  - [Features](#features)
  - [Usage](#usage)
  - [Examples](#examples)
    - [File transfer](#file-transfer)
    - [Host #1](#host-1)
    - [CI script with no SSH auth](#ci-script-with-no-ssh-auth)
    - [Evaluate compression](#evaluate-compression)
    - [mDNS utilities](#mdns-utilities)
      - [Discover services](#discover-services)
      - [Resolve mDNS hostname](#resolve-mdns-hostname)
      - [Register mDNS service (for testing or transferring by addressing the registered hostname)](#register-mdns-service-for-testing-or-transferring-by-addressing-the-registered-hostname)
  - [Supported compression formats](#supported-compression-formats)
  - [Install](#install)
    - [Prebuilt binaries](#prebuilt-binaries)
    - [Comparison/Benchmarks](#comparisonbenchmarks)

## Purpose

Transfer files as **quickly**, **safely**, and **painlessly** as possible on a local network.

`qft` optimizes for a scenario where embedded systems regularly transfer large files across a local network, such as a continuous integration pipeline where firmware (e.g. Rauc) can take significant time to transfer with tools such as `rsync`, `scp`, or `netcat`.

To accomplish this, `qft` acts as a server/client that transfers data over TCP. It is very similar to how `netcat` can be used to transfer files, but `qft` focuses solely on transferring files, and comes with a variety of customization options such as [compression/decompression](#supported-compression-formats), memory mapping, preallocation options and more. TCP is chosen for reliable data transfer, and no authentication or encryption is layered on top to reduce the overhead, addressing remote targets by mDNS is also supported.

If you are worried about a man-in-the-middle, you can simply check your data on the receiving end before continuing. There should be no additional security concerns (if you disagree, please create an issue highlighting the concern).

## Features

- [x] Send files via TCP by specifying either IP or mDNS/DNS-SD hostname
- [x] Evaluate [supported compression formats](#supported-compression-formats) on your input data
- [x] Discover, resolve, and/or register mDNS/DNS-SD services
- [x] SCP like transfers `qft send ssh <user>@<host>:<path> --file f.txt`. Where auth occurs via SSH but transfer is bare bone TCP.
- [x] Shell completions for bash, elvish, fish, powershell, and zsh.

## Usage

```markdown
$ qft -h
Usage: qft [OPTIONS] [COMMAND]

Commands:
  listen                Run in Listen (server) mode
  send                  Run in Send (client) mode
  mdns                  Use mDNS utilities
  evaluate-compression  Evaluate which compression works best for file content
  get-free-port         Get a free port from the host OS. Optionally specify on which IP or a port range to scan for a free port
  help                  Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose...           Pass many times for more log output
  -q, --quiet                Silence all log output, this will lead to better performance [env: QFT_QUIET=]
      --color=<WHEN>         [default: auto] [possible values: auto, always, never]
      --completions <SHELL>  Generate completion scripts for the specified shell. Note: The completion script is printed to stdout [possible values: bash, elvish, fish, powershell, zsh]
  -h, --help                 Print help (see more with '--help')
  -V, --version              Print version
```

## Examples

### File transfer

In a CI script using key based SSH auth, it looks very similar to SCP.
> Both hosts need qft installed!

### Host #1

```shell
qft send ssh foo@bar.local:/tmp/data --file received.data
```

### CI script with no SSH auth

Something like a Raspberry Pi could orchestrate the testing of an embedded system, and might use a script like this to transfer a firmware upgrade bundle.

```bash
#!/usr/bin/env bash
set -eu
HOST1_HOSTNAME="foo.local."
FIRMWARE="fw.raucb"
ssh -f user@${HOST1_HOSTNAME} "sh -c 'nohup qft listen --file ${FIRMWARE} > qft_listen.log 2>&1 &'"
qft send mdns ${HOST1_HOSTNAME} --file ${FIRMWARE} --prealloc
ssh user@${HOST1_HOSTNAME} -t "rauc install ${FIRMWARE}"
...
```

It is also possible to ad-hoc register a service with `qft mdns register` AND run the `qft listen` side-by-side and then send to the listening process by addressing the registered hostname from a remote host.

### Evaluate compression

```markdown
Evaluate which compression works best for file content

Usage: qft evaluate-compression [OPTIONS] --input-file <INPUT_FILE>

Options:
  -i, --input-file <INPUT_FILE>
      --omit [<OMIT>...]                List of compression formats to omit from evaluation [possible values: bzip2, gzip, lz4, xz]
      --omit-levels [<OMIT_LEVELS>...]  List of compression levels to omit from evaluation
  -v, --verbose...                      Pass many times for more log output
  -q, --quiet                           Silence all output [env: QFT_QUIET=]
      --color=<WHEN>                    [default: auto] [possible values: auto, always, never]
  -h, --help                            Print help (see more with '--help')
```

Evaluate compression of `Cargo.lock`. Omit `gzip` and most compression levels to make this example brief.

```shell
qft evaluate-compression --input-file Cargo.lock --omit gzip --omit-levels 0 2 3 4 5 6 7 8
```

Example output:

```shell
INFO Omitting:   Gzip
INFO Evaluating: Bzip2 Lz4 Xz
INFO Omitting compression levels (where applicable): 0 2 3 4 5 6 7 8
INFO Buffered reading 34338 B contents in 15.728µs
INFO Lz4
╭────────────────────┬─────────────────────╮
│ Compression Ratio  ┆        2.42:1       │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ Encode/decode time ┆   83.29µs/32.53µs   │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ Compressed Size    ┆ 13.83 KiB [14163 B] │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ % of Original      ┆        41.25%       │
╰────────────────────┴─────────────────────╯
INFO Bzip2
╭────────────────────┬───────────────────╮
│ Compression level  ┆         1         │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ Compression Ratio  ┆       4.56:1      │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ Encode/decode time ┆  2.36ms/686.34µs  │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ Compressed Size    ┆ 7.36 KiB [7533 B] │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ % of Original      ┆       21.94%      │
╰────────────────────┴───────────────────╯
INFO Bzip2
╭────────────────────┬───────────────────╮
│ Compression level  ┆         9         │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ Compression Ratio  ┆       4.56:1      │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ Encode/decode time ┆  2.73ms/771.13µs  │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ Compressed Size    ┆ 7.36 KiB [7533 B] │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ % of Original      ┆       21.94%      │
╰────────────────────┴───────────────────╯
INFO Xz
╭────────────────────┬───────────────────╮
│ Compression level  ┆         1         │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ Compression Ratio  ┆       3.73:1      │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ Encode/decode time ┆  2.34ms/522.16µs  │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ Compressed Size    ┆ 9.00 KiB [9216 B] │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ % of Original      ┆       26.84%      │
╰────────────────────┴───────────────────╯
INFO Xz
╭────────────────────┬───────────────────╮
│ Compression level  ┆         9         │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ Compression Ratio  ┆       4.30:1      │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ Encode/decode time ┆  9.04ms/523.23µs  │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ Compressed Size    ┆ 7.80 KiB [7984 B] │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ % of Original      ┆       23.25%      │
╰────────────────────┴───────────────────╯
%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%
%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%

╭────────────────────┬───────────────────┬───────────────────────┬─────────────────────────╮
│                    ┆ Best Ratio        ┆ Best Compression Time ┆ Best Decompression Time │
╞════════════════════╪═══════════════════╪═══════════════════════╪═════════════════════════╡
│ Format             ┆ Bzip2             ┆ Lz4                   ┆ Lz4                     │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ Compression level  ┆ 1                 ┆ -                     ┆ -                       │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ Compression Ratio  ┆ 4.56:1            ┆ 2.42:1                ┆ 2.42:1                  │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ Encode/decode time ┆ 2.36ms/686.34µs   ┆ 83.29µs/32.53µs       ┆ 83.29µs/32.53µs         │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ Compressed Size    ┆ 7.36 KiB [7533 B] ┆ 13.83 KiB [14163 B]   ┆ 13.83 KiB [14163 B]     │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ % of Original      ┆ 21.94%            ┆ 41.25%                ┆ 41.25%                  │
╰────────────────────┴───────────────────┴───────────────────────┴─────────────────────────╯

==> Short summary
Best Compression Ratio:   Bzip2[1] Compression/Decompression:     2.36ms/  686.34µs   4.56:1 (21.94% of original)
Best Compression Time:    Lz4      Compression/Decompression:    83.29µs/   32.53µs   2.42:1 (41.25% of original)
Best Decompression Time:  Lz4      Compression/Decompression:    83.29µs/   32.53µs   2.42:1 (41.25% of original)
```

### mDNS utilities

The purpose of the built-in mDNS/DNS-SD utilities are solely for easy network setup/testing/debugging, therefor they are generally more verbose and much slower than e.g. `avahi` is.

#### Discover services

```shell
qft mdns discover --service-label googlecast --service-protocol tcp
```

Example Output

```text
INFO Browsing for _googlecast._tcp.local.
INFO Resolved a new service: uie4027lgu-0b9b5630aa2b87f6945638a0128bfedd._googlecast._tcp.local.
INFO Discovered 1 service!
Hostname:  0b9b5670-aa2b-87d6-9456-38a0128bfedd.local.
Type Name: _googlecast._tcp.local.
Full Name: uie4027lgu-0b9b5670aa2b87d6945638a0128bfedd._googlecast._tcp.local.
IP(s): fe80::d912:463a:8c88:deca
       192.168.121.21
```

#### Resolve mDNS hostname

Resolves hostname IP(s), all of the following forms are valid.

```shell
qft mdns resolve foo
qft mdns resolve foo.local
qft mdns resolve foo.local.
```

Example output

```text 0b9b5670-aa2b-87d6-9456-38a0128bfedd
INFO Resolving address for 0b9b5670-aa2b-87d6-9456-38a0128bfedd.local.
Hostname:  0b9b5670-aa2b-87d6-9456-38a0128bfedd.local.
IP(s): fe80::d912:463a:8c88:deca
       192.168.121.21
```

#### Register mDNS service (for testing or transferring by addressing the registered hostname)

```shell
qft mdns register --hostname foo-name --service-label bar-label --service-protocol tcp --keep-alive-ms 123456
```

```text
INFO Registering:
    Hostname:  foo-name.local.
    Type:      _bar-label._tcp.local.
    Full Name: test_inst._bar-label._tcp.local.

INFO Keeping alive for: 123.456s
```

You can the find it using the `qft mdns` subcommands or e.g. with `avahi`:

```shell
avahi-resolve --name foo-name.local
# foo-name.local  172.17.0.1
```

But that only outputs the first received address. Using `qft mdns resolve` will output all the associated IPs. If you need speed use the `--short-circuit` flag to return as soon as the first IP associated with the hostname is resolved e.g.

```shell
qft mdns resolve foo-name[.local.] --short-circuit
```

## Supported compression formats

- [x] bzip2
- [x] gzip
- [x] lz4
- [x] xz

## Install

```shell
cargo install quick-file-transfer
```

### Prebuilt binaries

```shell
curl -L -H "Accept: application/vnd.github.v3.raw" \
        https://api.github.com/repos/CramBL/quick-file-transfer/contents/scripts/install.sh \
        | bash -s -- --to ~/bin
```


### Comparison/Benchmarks

Simple benchmark comparison with netcat on Ubuntu 22.04 to give an idea about transfer speeds. `f.raucb` is a 430MB file and the `&& sleep 1` allows the servers to spin down/up again after a completed transfer, that second is subtracted from the results table.

```shell
hyperfine  "qft send ip 127.0.0.1 --file testbundle.raucb && sleep 1" \
           "qft send ip 127.0.0.2 --prealloc --file testbundle.raucb && sleep 1" \
           "nc -N 0.0.0.0 1234 < testbundle.raucb && sleep 1"
```

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `qft send ip 127.0.0.1 --file f.raucb` | 218 ± 8 | 208 | 231 | 1.04 ± 0.01 |
| `qft send ip 127.0.0.2 --prealloc --file f.raucb` | 176 ± 4 | 174 | 185 | 1.00 |
| `nc -N 0.0.0.0 1234 < f.raucb` | 248 ± 4 | 241 | 252 | 1.06 ± 0.00 |
