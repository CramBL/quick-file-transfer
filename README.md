# Quick File Transfer (qft)

- [Quick File Transfer (qft)](#quick-file-transfer-qft)
  - [Purpose](#purpose)
  - [Usage](#usage)
  - [Examples](#examples)
    - [File transfer](#file-transfer)
    - [Host #1](#host-1)
    - [Host #2](#host-2)
  - [Example CI scrip](#example-ci-scrip)
    - [mDNS utilities](#mdns-utilities)
      - [Discover services](#discover-services)
      - [Resolve mDNS hostname](#resolve-mdns-hostname)
      - [Register mDNS service (for testing)](#register-mdns-service-for-testing)
  - [Supported compression formats](#supported-compression-formats)
  - [Install](#install)
    - [Prebuilt binaries](#prebuilt-binaries)

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
  listen  Run in Listen (server) mode
  send    Run in Send (client) mode
  mdns    Use mDNS utilities
  help    Print this message or the help of the given subcommand(s)

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

## Example CI scrip

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

- [ ] bz2
- [x] gzip
- [x] lz4
- [ ] xz

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
