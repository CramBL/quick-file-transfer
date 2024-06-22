ARMV7_MUSL_DEBUG_BIN := "target/armv7-unknown-linux-musleabihf/debug/qft"
x86_64_LINUX_MUSL_DEBUG_BIN := "target/x86_64-unknown-linux-musl/debug/qft"

[group("Cross Compile"), no-exit-message]
setup-armv7-musl-deps:
	rustup target add armv7-unknown-linux-musleabihf

[group("Cross Compile"), no-exit-message]
build-armv7-musl:
	cargo build --target=armv7-unknown-linux-musleabihf


[group("Cross Compile"), no-exit-message]
setup-x86_64-linux-musl-deps:
	#!/usr/bin/env bash
	set -euo pipefail
	rustup target add armv7-unknown-linux-musleabihf
	if ! just is-installed musl-gcc; then
		if ! just is-installed apt-get; then
			{{ERROR}} "musl-gcc is needed, install it for your system (hint: named musl-tools?)"
		else
			sudo apt-get update
			sudo apt-get install musl-tools
			musl-gcc --version
		fi
	fi

[group("Cross Compile"), no-exit-message]
build-x86_64-linux-musl $RUSTFLAGS="" *ARGS="": setup-x86_64-linux-musl-deps
	cargo build --target=x86_64-unknown-linux-musl {{ARGS}}
