# Quick File Transfer (qft)

- [Quick File Transfer (qft)](#quick-file-transfer-qft)
  - [Purpose](#purpose)
  - [Features](#features)
  - [Usage](#usage)
  - [Examples](#examples)
    - [File transfer](#file-transfer)
    - [Host #1](#host-1)
    - [Host #2](#host-2)
    - [CI script](#ci-script)
    - [Evaluate compression](#evaluate-compression)
    - [mDNS utilities](#mdns-utilities)
      - [Discover services](#discover-services)
      - [Resolve mDNS hostname](#resolve-mdns-hostname)
      - [Register mDNS service (for testing)](#register-mdns-service-for-testing)
  - [Supported compression formats](#supported-compression-formats)
  - [Install](#install)
    - [Prebuilt binaries](#prebuilt-binaries)
    - [Comparison/Benchmarks](#comparisonbenchmarks)

## Purpose

Transfer files as **quickly**, **safely**, and **painlessly** as possible on a local network.

`qft` optimizes for a scenario where embedded systems regularly transfer large files across a local network, such as a continuous integration pipeline where firmware (e.g. Rauc) can take significant time to transfer with tools such as `rsync`, `scp`, or `netcat`.

To accomplish this, `qft` acts as a server/client that transfers data over TCP. It is very similar to how `netcat` can be used to transfer files, but `qft` focuses solely on transferring files, and comes with a variety of customization options such as [compression/decompression](#supported-compression-formats), memory mapping, preallocation options and more. TCP is chosen for reliable data transfer, and no authentication or encryption is layered on top to reduce the overhead.

If you are worried about a man-in-the-middle, you can simply check your data on the receiving end before continuing. There should be no additional security concerns (if you disagree, please create an issue highlighting the concern).

## Features

- [x] Send files via TCP by specifying either IP or mDNS/DNS-SD hostname
- [x] Evaluate [supported compression formats](#supported-compression-formats) on your input data
- [x] Discover, resolve, and/or register mDNS/DNS-SD services

## Usage

```markdown
$ qft -h
Usage: qft [OPTIONS] <COMMAND>

Commands:
  listen                Run in Listen (server) mode
  send                  Run in Send (client) mode
  mdns                  Use mDNS utilities
  evaluate-compression  Evaluate which compression works best for file content
  help                  Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose...  Pass many times for more log output
  -q, --quiet       Silence all output [env: QFT_QUIET=]
  -h, --help        Print help (see more with '--help')
  -V, --version     Print version
```

## Examples

### File transfer

In a CI script Host #2 could simply ssh into Host #1 and launch the `qft listen` command as a background process before invoking `qft send`.

### Host #1

Listen on port `1234`.

```shell
qft listen --ip 0.0.0.0 --port 12345 --file received.data
```

### Host #2

Transfer a file to **Host #1**.

```shell
qft send ip <HOST-1-IP> --port 12345 --file transfer.data
```

### CI script

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

Usage: qft evaluate-compression [OPTIONS] --input-file <INPUT_FILE> [OMIT]...

Arguments:
  [OMIT]...  List of compression formats to omit from evaluation [possible values: gzip, bzip2, xz, lz4]

Options:
  -i, --input-file <INPUT_FILE>
      --test-mmap                Also test with memory mapping
  -v, --verbose...               Pass many times for more log output
  -q, --quiet                    Silence all output [env: QFT_QUIET=]
  -h, --help                     Print help (see more with '--help')
```

Evaluate compression of `Cargo.lock`.

```shell
qft evaluate-compression --input-file Cargo.lock`
```

Example output:

```shell
evaluating: Gzip
evaluating: Bzip2
evaluating: Xz
evaluating: Lz4
Buffered reading 30970 B contents in 17.771µs
Gzip
    Ratio: 3.82:1
    Compression Time:    556.61µs
    Decompression Time:  123.52µs
    Size:  7.91 KiB [8097 B] (26.14% of original)

Bzip2
    Ratio: 4.52:1
    Compression Time:    2.17ms
    Decompression Time:  588.87µs
    Size:  6.69 KiB [6848 B] (22.11% of original)

Xz
    Ratio: 4.27:1
    Compression Time:    8.36ms
    Decompression Time:  493.73µs
    Size:  7.08 KiB [7252 B] (23.42% of original)

Lz4
    Ratio: 2.42:1
    Compression Time:    44.65µs
    Decompression Time:  20.10µs
    Size:  12.49 KiB [12791 B] (41.30% of original)

===> Summary
Best Compression Ratio:   Bzip2 Compression/Decompression:     2.17ms/  588.87µs   4.52:1 (22.11% of original)
Best Compression Time:    Lz4   Compression/Decompression:    44.65µs/   20.10µs   2.42:1 (41.30% of original)
Best Decompression Time:  Lz4   Compression/Decompression:    44.65µs/   20.10µs   2.42:1 (41.30% of original)

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

#### Register mDNS service (for testing)

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

But that only outputs the first received address. Using `qft mdns resolve` will always output all the associated IPs. If you need speed, `avahi` is a much better choice though.

## Supported compression formats

- [x] bz2
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
