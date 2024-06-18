#!/usr/bin/env -S just --quiet --justfile
## Cheatsheet: https://en.wikipedia.org/wiki/ANSI_escape_code#Colors
## Contains utilities for printing colored text to the terminal for distro-builder-common
##
## NOTE: the "PRINT" variable is defined by distro-template so here we use "ECHO" instead to avoid multiple definitions.
##
## The "ECHO" variable is defined in the top-level justfile
##  and is an absolute path to this file:
## ECHO := join(justfile_directory(), "util-scripts/just-util/pretty_print.just")
##  thus it can be used to call the print function from any justfile in the project, in any directory,
##  making it immune to cd-ing around in Bash/Python/etc. recipes.
##
## Usage:
##    {{ECHO}} green "Success!"
##    {{ECHO}} cyan "Info"
## By default, a newline is appended to the end of the text. To suppress this, set the NEWLINE variable to something other than "true"
##    {{ECHO}} NEWLINE="no" green "Success!"
##
# Start of ANSI escape sequence

ANSI_ESC := '\x1b['
ANSI_ESC_CLR := ANSI_ESC + "0m"
ANSI_BOLD_GREEN := ANSI_ESC + '1;32m'
ANSI_BOLD_CYAN := ANSI_ESC + '1;36m'
ANSI_BOLD_YELLOW := ANSI_ESC + '1;33m'
ANSI_BOLD_RED := ANSI_ESC + '1;31m'
ANSI_BOLD_MAGENTA := ANSI_ESC + '1;35m'
ANSI_BOLD_BLUE := ANSI_ESC + '1;34m'

EFFECT_BOLD := ';1'
EFFECT_UNDERLINE := ';4'

NEWLINE := 'true'

green TEXT: (print ANSI_BOLD_GREEN TEXT)
cyan TEXT: (print ANSI_BOLD_CYAN TEXT)
yellow TEXT: (print ANSI_BOLD_YELLOW TEXT)
red TEXT: (print ANSI_BOLD_RED TEXT)
magenta TEXT: (print ANSI_BOLD_MAGENTA TEXT)
blue TEXT: (print ANSI_BOLD_BLUE TEXT)

# Generic print function
[private]
print ANSI_START TEXT:
    #!/usr/bin/env bash
    declare -r optional_newline=$( [ {{ NEWLINE }} == 'true' ] && echo "\n" || echo "" )
    printf "%b%b%b${optional_newline}" "{{ ANSI_START }}" "{{ TEXT }}" "{{ ANSI_ESC_CLR }}"

[private]
print_rgb R G B TEXT EFFECT="":
    #!/usr/bin/env bash
    set -euo pipefail
    PREFIX="{{ANSI_ESC}}38;2;{{ R }};{{ G }};{{ B }}{{ EFFECT }}m"
    printf "%b%b%b" "${PREFIX}" "{{ TEXT }}" "{{ ANSI_ESC_CLR }}"

# Prints a success message in light green
[private]
success TEXT:
	#!/usr/bin/env bash
	printf "%b%b%b\n" "{{ANSI_ESC}}38;2;20;170;20m" "{{TEXT}}" "{{ANSI_ESC_CLR}}"

# Prints an error message in red and exits with code 1
[private, no-exit-message]
error TEXT EXIT_CODE="1":
	#!/usr/bin/env bash
	printf "%b%b%b\n" "{{ANSI_BOLD_RED}}" "{{TEXT}}" "{{ANSI_ESC_CLR}}" >&2
	exit {{EXIT_CODE}}

# Prints a warning message in yellow
[private]
warning TEXT:
	#!/usr/bin/env bash
	printf "%b%b%b\n" "{{ANSI_BOLD_YELLOW}}" "{{TEXT}}" "{{ANSI_ESC_CLR}}" >&2

# Prints RGB text with interpolated variables that are printed in white
# e.g. print_rgb_interpolated 255 255 255 "Hi %0 and %1\n" "Alice" "Bob"
[private]
print_rgb_interpolated R G B TEXT V0="" V1="" V2="" V3="" V4="" V5="" V6="" V7="" V8="" V9="":
	#!/usr/bin/env bash
	set -euo pipefail
	PREFIX="{{ANSI_ESC}}38;2;{{ R }};{{ G }};{{ B }}m"
	TEXT="{{TEXT}}"
	# Replace all the variables specified as %N with their values
	declare -ar VARS=("{{V0}}" "{{V1}}" "{{V2}}" "{{V3}}" "{{V4}}" "{{V5}}" "{{V6}}" "{{V7}}" "{{V8}}" "{{V9}}")
	for idx in "${!VARS[@]}"; do
		# The replacement has to first escape the ANSI escape sequence
		# And then insert the prefix again after the replacement
		replacement="{{ANSI_ESC_CLR}}${VARS[${idx}]}${PREFIX}"
		TEXT=${TEXT//"%${idx}"/${replacement}}
	done
	printf "%b%b%b" "${PREFIX}" "${TEXT}" "{{ANSI_ESC_CLR}}"

