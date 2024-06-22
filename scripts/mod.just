import 'default_vars.just'
import 'check_version.just'
import 'git.just'
import 'docker.just'
import 'cross.just'


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
