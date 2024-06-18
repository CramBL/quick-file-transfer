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
