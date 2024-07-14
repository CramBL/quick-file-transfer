<div align=right>Table of Contents↗️</div>

<h1 align=center>Quick File Transfer

<code>qft</code>

</h1>


<div align=center>
  <a href=https://crates.io/crates/quick-file-transfer>
    <img src=https://img.shields.io/crates/v/quick-file-transfer.svg alt="crates.io version">
  </a>
  <a href=https://github.com/CramBL/quick-file-transfer/actions>
    <img src=https://github.com/CramBL/quick-file-transfer/actions/workflows/CI.yml/badge.svg alt="build status">
  </a>
  <a href=https://github.com/CramBL/just/releases>
    <img src=https://img.shields.io/github/downloads/CramBL/quick-file-transfer/total.svg alt=downloads>
  </a>
</div>
<br>


## Purpose

> Note! This is a work in progress and is in no means production ready

Transfer files as **quickly**, **safely**, and **painlessly** as possible on a local network.

This readme is also available as a [book](https://crambl.github.io/quick-file-transfer/man/).

`qft` optimizes for a scenario where embedded systems regularly transfer large files across a local network, such as a continuous integration pipeline where firmware (e.g. Rauc) can take significant time to transfer with tools such as `rsync`, `scp`, or `netcat`.

To accomplish this, `qft` acts as a server/client that transfers data over TCP. It is very similar to how `netcat` can be used to transfer files, but `qft` focuses solely on transferring files, and comes with a variety of customization options such as [compression/decompression](#supported-compression-formats), memory mapping, preallocation options and more. TCP is chosen for reliable data transfer, and no authentication or encryption is layered on top to reduce the overhead, addressing remote targets by mDNS is also supported.

If you are worried about a man-in-the-middle, you can simply check your data on the receiving end before continuing. There should be no additional security concerns (if you disagree, please create an issue highlighting the concern).

## Features

* Send files via TCP by specifying either IP or mDNS/DNS-SD hostname
* Evaluate [supported compression formats](#supported-compression-formats) on your input data
* Discover, resolve, and/or register mDNS/DNS-SD services
* SCP like transfers `qft ssh FILES... <user>@<host>:<path>`. Where auth occurs via SSH but transfer is bare bone TCP.
* Shell completions for `bash`, `elvish`, `fish`, `powershell`, and `zsh`.

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

#### File transfer

In a CI script using key based SSH auth, it looks very similar to SCP.
> Both hosts need qft installed!

#### Host #1

```shell
qft ssh file.data foo@bar.local:/tmp/
```

#### CI script with no SSH auth

Something like a Raspberry Pi could orchestrate the testing of an embedded system, and might use a script like this to transfer a firmware upgrade bundle.

```bash
#!/usr/bin/env bash
set -eu
REMOTE_HOSTNAME="foo.local."
FIRMWARE="fw.raucb"
qft ssh ${FIRMWARE} root@${REMOTE_HOSTNAME}:/
ssh root@${REMOTE_HOSTNAME} -t "rauc install /${FIRMWARE}"
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

#### Demo

<div align=center>
    <img src=www/evaluate_compression_demo.gif alt="evaluate-compression demo">
</div>

#### Example result

```shell
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

The purpose of the built-in mDNS/DNS-SD utilities are solely for easy network setup/testing/debugging, therefor they are generally more verbose and have much slower (but more complete) defaults than e.g. `avahi` does.

#### Discover services

```shell
qft mdns discover --service-label googlecast --service-protocol tcp
```

Example Output

```text
INFO Browsing for _googlecast._tcp.local.
INFO Resolved a new service: SERVICE_NAME._googlecast._tcp.local.
INFO Discovered 1 service!
Hostname:  SERVICE_NAME.local.
Type Name: _googlecast._tcp.local.
Full Name: SERVICE_NAME._googlecast._tcp.local.
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

```text
INFO Resolving address for foo.local.
Hostname:  foo.local.
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

<ul>
<li><input checked="" disabled="" type="checkbox"> bzip2</li>
<li><input checked="" disabled="" type="checkbox"> gzip</li>
<li><input checked="" disabled="" type="checkbox"> lz4</li>
<li><input checked="" disabled="" type="checkbox"> xz</li>
</ul>


## Install

Build from source (preferred if you have the Rust toolchain installed).

```shell
cargo install quick-file-transfer
```

#### Prebuilt binaries

```shell
curl --proto '=https' --tlsv1.2 -sSf https://crambl.github.io/quick-file-transfer/install.sh | bash -s -- --to <DEST>
```

## Comparison/Benchmarks

Benchmarks are done with `hyperfine` default settings.

Using a 7.2 MiB JSON-file (not prettified) I had nearby with real data.

Targeting a `Raspberry Pi Zero W` that is connected with ethernet to a gigabit network.

#### netcat-like mode

The RPI-0W was running `qft listen -p <PORT> --output 7mb.json` and `nc -l <PORT> > 7mb.json` on repeat for the duration of the benchmark.

| Command                                       | Mean [ms] | Min [ms] | Max [ms] |    Relative |
| :-------------------------------------------- | --------: | -------: | -------: | ----------: |
| `qft send ip <IP> -p <PORT> -f 7mb.json`      |  895 ± 52 |      766 |      935 | 1.31 ± 0.11 |
| `qft send ip <IP> -p <PORT> -f 7mb.json lz4`  |  486 ± 60 |      404 |      584 | 1.03 ± 0.09 |
| `qft send ip <IP> -p <PORT> -f 7mb.json gzip` | 444 ± 120 |      325 |      685 |        1.00 |
| `nc -N <IP> <PORT> < 7mb.json`                |  1049 ± 8 |     1036 |     1056 | 1.42 ± 0.12 |

#### scp-like mode

| Command                                          |     Mean [ms] | Min [ms] | Max [ms] |    Relative |
| :----------------------------------------------- | ------------: | -------: | -------: | ----------: |
| `qft ssh 7mb.json <user>@<IP>:~/7mb.json`        | 2.295 ± 0.131 |    2.108 |    2.399 | 1.14 ± 0.10 |
| `qft ssh 7mb.json <user>@<IP>:~/7mb.json lz4`    | 2.004 ± 0.122 |    1.797 |    2.194 |        1.00 |
| `qft ssh 7mb.json <user>@<IP>:~/7mb.json gzip 4` | 2.026 ± 0.120 |    1.847 |    2.122 | 1.01 ± 0.09 |
| `scp 7mb.json <user>@<IP>:~/7mb.json`            | 2.448 ± 0.039 |    2.404 |    2.532 | 1.22 ± 0.08 |
