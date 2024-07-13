# Absolute path to the directory containing the utility recipes to invoke them from anywhere
## USAGE: `{{ECHO}} green "Hello world"`
ECHO := join(justfile_directory(), "scripts/dont_import/pretty_print.just")
## Same as ECHO but takes rgb values as arguments and does NOT print a newline by default
ECHO_RGB := ECHO + " print_rgb"
# Prints the argument in red to stderr and exits with status code 1. The status code can be overwritten with the optional second argument.
ERROR := ECHO + " error"
# Prints the argument in yellow to stderr
WARN := ECHO + " warning"
# Prints the argument in green to stdout
SUCCESS := ECHO + " success"
# Prints with the specified RGB color to stdout and allows for interpolation of variables with %0, %1, %2, etc.
# e.g. {{PRINT_RGB}} "Hello %0, this is %1" "world" "a test"
PRINT_RGB := ECHO + " print_rgb_interpolated"

cargo_binaries := join(home_directory(), ".cargo/bin/")

check := if path_exists(join(cargo_binaries, "cargo-lcheck")) == "true" { "lcheck" } else { "check" }
clippy := if path_exists(join(cargo_binaries, "cargo-lclippy")) == "true" { "lclippy" } else { "clippy" }
run := if path_exists(join(cargo_binaries, "cargo-lrun")) == "true" { "lrun" } else { "run" }
build := if path_exists(join(cargo_binaries, "cargo-lbuild")) == "true" { "lbuild" } else { "build" }
doc := if path_exists(join(cargo_binaries, "cargo-ldoc")) == "true" { "ldoc" } else { "doc" }
USING_NEX_TEST := if path_exists(join(cargo_binaries, "cargo-nextest")) == "true" { "true" } else { "false" }
USING_LTEST := if path_exists(join(cargo_binaries, "cargo-ltest")) == "true" { "true" } else { "false" }
test := if USING_NEX_TEST == "true" {
	"nextest run"
} else if USING_LTEST == "true" {
	"ltest"
} else {
	"test"
}
# Needs to be specified before '--', i.e. cargo nextest run --run-ignored all -- ...
nex_include_ignored_tests := if USING_NEX_TEST == "true" {
	"--run-ignored all"
} else {
	""
}
# Needs to be specified after '--', i.e. cargo test -- --ignored
include_ignored := if USING_NEX_TEST == "false" {
	"--ignored"
} else {
	""
}

TEST_THREADS := if USING_NEX_TEST == "true" {
	"NEXTEST_TEST_THREADS"
} else {
	"RUST_TEST_THREADS"
}
