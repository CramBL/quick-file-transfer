import 'scripts/mod.just'

@_default:
    just --list --no-aliases

alias l := lint
alias c := check
alias f := format
alias t := test

# Needs the rust toolchain
env:
    rustc --version
    cargo --version

# Lint the code
[no-exit-message]
lint *ARGS="-- -D warnings --no-deps":
    cargo clippy {{ ARGS }}

# Run pre-commit on all files
[no-exit-message]
run-pre-commit:
    pre-commit run --all-files

# Format the code
[no-exit-message]
format *ARGS:
    cargo fmt {{ ARGS }}

# Check if it compiles without compiling
[no-exit-message]
check *ARGS:
    cargo check {{ ARGS }}

# Run the tests
[no-exit-message]
test *ARGS:
    cargo test {{ ARGS }}

    # Build the application
build *ARGS:
    cargo build {{ ARGS }}

# Run the application (use `--` to pass arguments to the application)
run ARGS:
    cargo run {{ ARGS }}

# Clean the `target` directory
clean:
    cargo clean

# Build the documentation (use `--open` to open in the browser)
doc *ARGS:
    cargo doc {{ ARGS }}

# Publish the crate
publish:
    cargo publish

# List the dependencies
deps:
    cargo tree

# Update the dependencies
update:
    cargo update

# Audit Cargo.lock files for crates containing security vulnerabilities
audit *ARGS:
    #!/usr/bin/env bash
    if ! which cargo-audit >/dev/null; then
        {{ECHO}} yellow "cargo-audit not found"
        just prompt-install "cargo install cargo-audit"
    fi
    cargo audit {{ ARGS }}

## CI specific recipes (run these to check if the code passes CI)
ci-lint: \
    (check "--verbose") \
    (lint "--verbose -- -D warnings --no-deps") \
    (format "-- --check --verbose") \
    (doc "--verbose --no-deps") \
    check-version \