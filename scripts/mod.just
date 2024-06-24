import 'default_vars.just'
import 'check_version.just'
import 'git.just'
import 'cross.just'
import 'docker/mod.just'

# Checks if BINARY is a command/is installed and exits 1 if not
[private, no-exit-message]
is-installed BINARY:
	#!/usr/bin/env bash
	set -euo pipefail
	if ! command -v "{{BINARY}}" > /dev/null 2>&1; then
		echo "Not installed: {{BINARY}} (command not found)"
		exit 1
	fi

# Same as above but returns the result as a string
[private, no-exit-message]
installed BINARY:
	#!/usr/bin/env bash
	set -euo pipefail
	if ! command -v "{{BINARY}}" > /dev/null 2>&1; then
		echo "true"
	else
		echo "false"
	fi


# Checks if the recipe is running in a terminal
[private, no-exit-message]
is-tty:
	#!/usr/bin/env sh
	if [ -t 1 ]; then
		exit 0
	else
		exit 1
	fi
