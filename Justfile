import 'scripts/mod.just'

set shell := ["bash", "-uc"]

@_default:
    just --list --no-aliases

alias l := lint
alias c := check
alias f := format
alias t := test
alias p := pre-commit

# List tool version
[unix]
env:
    just --version
    rustc --version         || echo "Not found"
    cargo --version         || echo "Not found"
    cargo clippy --version  || echo "Not found"
    docker version          || echo "Not found"
    containerd --version    || echo "Not found"
    python --version        || echo "Not found"
    pip --version           || echo "Not found"
    ssh -V                  || echo "Not found"

# Lint the code
[group("Code Quality"), no-exit-message]
lint *ARGS="--all --all-targets -- -D warnings --no-deps":
    cargo {{clippy}} {{ ARGS }}

# Lint all feature combinations
[group("Code Quality"), no-exit-message]
lint-feature-combination *ARGS:
    cargo hack {{clippy}} --feature-powerset --no-dev-deps {{ARGS}}

# Run pre-commit and formatting/linting
[group("Code Quality"), no-exit-message]
pre-commit:
    pre-commit run
    cargo fmt
    cargo {{check}}
    cargo {{clippy}}
    cargo {{doc}}

# Format the code
[group("Code Quality"), no-exit-message]
format *ARGS:
    cargo fmt {{ ARGS }}

# Check if it compiles without compiling
[group("Code Quality"), no-exit-message]
check *ARGS:
    cargo {{check}} {{ ARGS }}

# Check all feature combinations
[group("Code Quality"), no-exit-message]
check-feature-combination *ARGS:
    cargo hack {{check}} --feature-powerset --no-dev-deps {{ARGS}}

# Build the application
[no-exit-message]
build *ARGS:
    cargo {{build}} {{ ARGS }}

# Run the application (use `--` to pass arguments to the application)
[no-exit-message]
run ARGS:
    cargo {{run}} {{ ARGS }}

# Clean the `target` directory
clean:
    cargo clean

# Build the documentation (use `--open` to open in the browser)
[group("Code Quality"), no-exit-message]
doc *ARGS:
    cargo {{doc}} {{ ARGS }}

# Publish the crate
[no-exit-message]
publish:
    cargo publish

# List the dependencies
[group("Dependencies")]
deps:
    cargo tree

# Update the dependencies
[group("Dependencies")]
update:
    cargo update

# Audit Cargo.lock files for crates containing security vulnerabilities
[group("Dependencies")]
audit *ARGS:
    #!/usr/bin/env bash
    if ! which cargo-audit >/dev/null; then
        {{ECHO}} yellow "cargo-audit not found"
        just prompt-install "cargo install cargo-audit"
    fi
    cargo audit {{ ARGS }}

## CI specific recipes (run these to check if the code passes CI)
[group("Code Quality")]
ci-lint: \
    (check "--verbose") \
    (lint "--verbose --all --all-targets -- -D warnings --no-deps") \
    (format "-- --check --verbose") \
    (doc "--verbose --no-deps")
