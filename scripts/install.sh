#!/usr/bin/env sh

## Adapted from https://github.com/casey/just/blob/610aa0c52cf8c3d20a79ee641bb9f799ca3027fc/www/install.sh

set -eu

# Echo commands if the GITHUB_ACTIONS variable is set and INSTALL_QUIET is NOT set.
if [ -n "${GITHUB_ACTIONS-}" ] && [ -z "${INSTALL_QUIET-}" ]; then
    set -x
fi

# Check pipefail support in a subshell, ignore if unsupported
# shellcheck disable=SC3040
(set -o pipefail 2> /dev/null) && set -o pipefail

help() {
    cat <<'EOF'
Install a binary release of a Quick File Transfer (qft) hosted on GitHub

USAGE:
    install [options]

FLAGS:
    -h, --help      Display this message
    -f, --force     Force overwriting an existing binary

OPTIONS:
    --tag TAG       Tag (version) of the crate to install, defaults to latest release
    --to LOCATION   Where to install the binary [default: ~/bin]
    --target TARGET
EOF
}

CRATE=quick-file-transfer
URL=https://github.com/CramBL/quick-file-transfer
RELEASES=${URL}/releases
BIN_NAME=qft

say() {
    echo "install: $*" >&2
}

err() {
    if [ -n "${td-}" ]; then
        rm -rf "$td"
    fi

    say "error: $*"
    exit 1
}

need() {
    if ! command -v "$1" > /dev/null 2>&1; then
        err "need $1 (command not found)"
    fi
}

download() {
    url="$1"
    output="$2"

    if command -v curl > /dev/null; then
        curl --proto =https --tlsv1.2 -sSfL "$url" "-o$output"
    else
        wget --https-only --secure-protocol=TLSv1_2 --quiet "$url" "-O$output"
    fi
}

install_bin() {
    src="$1"
    dst="$2"
    if command -v install > /dev/null; then
        install -m 755 "$src" "$dest"
    else
        cp "$src" "$dst"
        chmod 755 "$dst"
    fi
}

force=false
while test $# -gt 0; do
    case $1 in
        --force | -f)
            force=true
        ;;
        --help | -h)
            help
            exit 0
        ;;
        --tag)
            tag=$2
            shift
        ;;
        --target)
            target=$2
            shift
        ;;
        --to)
            dest=$2
            shift
        ;;
        *)
        ;;
    esac
    shift
done

need curl
need mkdir
need mktemp

if [ -z "${tag-}" ]; then
    need grep
    need cut
fi

if [ -z "${target-}" ]; then
    need cut
fi

if [ -z "${dest-}" ]; then
    dest="$HOME/bin"
fi

if [ -z "${tag-}" ]; then
    tag=$(
        download https://api.github.com/repos/Crambl/quick-file-transfer/releases/latest - |
            grep tag_name |
            cut -d'"' -f4
    )
fi

if [ -z "${target-}" ]; then
    # bash compiled with MINGW (e.g. git-bash, used in github windows runners),
    # unhelpfully includes a version suffix in `uname -s` output, so handle that.
    # e.g. MINGW64_NT-10-0.19044
    kernel=$(uname -s | cut -d- -f1)
    uname_target="$(uname -m)-$kernel"

    case $uname_target in
        aarch64-Linux)     target=aarch64-unknown-linux-musl;;
        armv7l-Linux)      target=armv7-unknown-linux-musleabihf;;
        armv6l-Linux)      target=arm-unknown-linux-musleabihf;;
        arm64-Darwin)      target=aarch64-apple-darwin;;
        x86_64-Darwin)     target=x86_64-apple-darwin;;
        x86_64-Linux)      target=x86_64-unknown-linux-musl;;
        x86_64-MINGW64_NT) target=x86_64-pc-windows-msvc;;
        x86_64-Windows_NT) target=x86_64-pc-windows-msvc;;
        *)
            # shellcheck disable=SC2016
            err 'Could not determine target from output of `uname -m`-`uname -s`, please use `--target`:' "$uname_target"
        ;;
    esac
fi

case $target in
    x86_64-pc-windows-msvc) extension=zip; need unzip;;
    *)                      extension=tar.gz; need tar;;
esac

archive="$RELEASES/download/$tag/$BIN_NAME-$tag-$target.$extension"

say "Repository:  $URL"
say "Crate:       $CRATE"
say "Binary name: $BIN_NAME"
say "Tag:         $tag"
say "Target:      $target"
say "Destination: $dest"
say "Archive:     $archive"

td=$(mktemp -d || mktemp -d -t tmp)

if [ "$extension" = "zip" ]; then
    download "$archive" "$td/${BIN_NAME}.zip"
    unzip -d "$td" "$td/${BIN_NAME}.zip"
else
    download "$archive" - | tar -C "$td" -xz
fi

if [ -e "$dest/${BIN_NAME}" ] && [ "$force" = false ]; then
    err "\`$dest/${BIN_NAME}\` already exists"
else
    mkdir -p "$dest"
    install_bin "$td/${BIN_NAME}" "$dest"
fi

rm -rf "$td"
