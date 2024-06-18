# Check that the version specified in Cargo.toml matches the latest git tag version
check-version:
    #!/usr/bin/env bash
    set -euo pipefail
    git fetch --tags || echo "Skipping fetch tags"
    cargo_ver=$( just _get_cargo_toml_version )
    git_tag_ver=$( just _latest_git_tag )
    if [[ ${cargo_ver} != ${git_tag_ver} ]]; then
        {{ERROR}} red "Mismatch between Cargo.toml and git tag version ${cargo_ver} != ${git_tag_ver}"
    fi

# Get the version as specified in Cargo.toml
_get_cargo_toml_version:
    grep --perl-regexp --only-matching "(?<=version = \")[0-9.]*" Cargo.toml | head -n+1

# Get the semver described by the latest tag
_latest_git_tag:
    #!/usr/bin/env bash
    if ! git describe --tags &> /dev/null; then
        echo "0.1.0"
        exit 0
    fi
    git describe --tags --abbrev=0 | grep --perl-regexp --only-matching "(?<=v).*"
