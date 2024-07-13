ARMV7_MUSL_DEBUG_BIN := "target/armv7-unknown-linux-musleabihf/debug/qft"
x86_64_LINUX_MUSL_DEBUG_BIN := "target/x86_64-unknown-linux-musl/debug/qft"



[group("Cross Compile"), no-exit-message]
build-armv7-musl *ARGS:
	cargo {{build}} --target=armv7-unknown-linux-musleabihf {{ARGS}}

[group("Cross Compile Dependencies"), no-exit-message]
setup-armv7-musl-deps:
	rustup target add armv7-unknown-linux-musleabihf

[group("Cross Compile"), no-exit-message]
build-aarch64-linux-musl *ARGS:
	cargo {{build}} --target=aarch64-unknown-linux-musl {{ARGS}}

[group("Cross Compile Dependencies"), no-exit-message]
setup-aarch64-linux-musl-deps:
	#!/usr/bin/env bash
	set -euo pipefail
	rustup target add aarch64-unknown-linux-musl
	if ! just is-installed gcc-aarch64-linux-gnu; then
		if ! just is-installed apt-get; then
			{{ERROR}} "gcc-aarch64-linux-gnu & libc6-dev-i386 is needed, install it for your system"
		else
			sudo apt-get update
			sudo apt-get install gcc-aarch64-linux-gnu libc6-dev-i386
		fi
	fi

[group("Cross Compile"), no-exit-message]
build-x86_64-linux-musl $RUSTFLAGS="" *ARGS="": setup-x86_64-linux-musl-deps
	cargo {{build}} --target=x86_64-unknown-linux-musl {{ARGS}}

[group("Cross Compile Dependencies"), no-exit-message]
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
