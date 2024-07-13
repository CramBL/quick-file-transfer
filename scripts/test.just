# Run the tests
[group("Test"), no-exit-message]
test *ARGS: && container-tests
	cargo {{test}} --bin qft -- {{ ARGS }}
	cargo {{test}} native_tests -- {{ ARGS }}

# Run the tests with all feature combinations
[group("Test"), no-exit-message]
test-feature-combination *ARGS:
    cargo hack {{test}} --feature-powerset {{ ARGS }}

# Run tests using the docker test container (includes the build step)
[group("Docker"), group("Test"), no-exit-message, linux]
d-test ARGS="": && d-setup-for-transfer-tests (container-tests ARGS)
    mkdir -m755 -p {{CONTAINER_MOUNTED_TMP_DIR}}

# Run tests targeting the docker container, requires building it first
[group("Docker"), group("Test"), no-exit-message, linux]
container-tests *ARGS:
	{{TEST_THREADS}}=1 \
		cargo {{test}} {{nex_include_ignored_tests}} container_tests \
			-- {{include_ignored}} {{ ARGS }}


[group("Docker"), group("Test"), no-exit-message, windows, macos]
container-tests *ARGS:
	{{WARN}} "Container tests are ignored on non-linux"
