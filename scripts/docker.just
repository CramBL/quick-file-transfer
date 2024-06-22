# Default image name
CONTAINER_TAG := "alpine-qft-ssh"
CONTAINER_USERNAME := "userfoo"
CONTAINER_PASSWORD := "password"
CONTAINER_HOME := "/home/" + CONTAINER_USERNAME

CONTAINER_QFT_LISTEN_PORT := "12999"
CONTAINER_SSH_PORT := "54320"

# Relaxed ssh options: 1. No host key checking 2. No known hosts file (avoids adding the target to known hosts) 3. Only log errors
SSH_OPTIONS_LAX := "\
-o StrictHostKeyChecking=no \
-o UserKnownHostsFile=/dev/null \
-o LogLevel=ERROR\
"

# Build the Docker image
[group("Docker"), no-exit-message]
d-build:
    docker build -t {{CONTAINER_TAG}} ./docker

# Run the Docker container
[group("Docker"), no-exit-message]
d-run:
	docker run --detach \
		--publish 127.0.0.1:{{CONTAINER_SSH_PORT}}:{{CONTAINER_SSH_PORT}} \
		--publish 127.0.0.1:{{CONTAINER_QFT_LISTEN_PORT}}:{{CONTAINER_QFT_LISTEN_PORT}} \
		--name {{CONTAINER_TAG}} {{CONTAINER_TAG}}


# Stop and remove the Docker container
[group("Docker"), no-exit-message]
d-stop:
	#!/usr/bin/env bash
	set -euo pipefail
	if just d-is-running; then
		docker stop {{CONTAINER_TAG}}
	fi
	if just d-is-exited; then
		docker rm {{CONTAINER_TAG}}
	fi

# SSH into the running container
[group("Docker"), no-exit-message]
d-ssh:
	ssh {{SSH_OPTIONS_LAX}} -p {{CONTAINER_SSH_PORT}} {{CONTAINER_USERNAME}}@localhost

# Check Docker logs
[group("Docker"), no-exit-message]
d-logs:
    docker logs {{CONTAINER_TAG}}

# Check if the Docker container is running
[group("Docker"), no-exit-message]
d-is-running:
	#!/usr/bin/env bash
	set -euo pipefail
	OUTPUT=$(docker ps --filter 'name=^/{{CONTAINER_TAG}}$$' --format '{{ "{{.Names}}: {{.Status}}" }}')
	if [[ -n "${OUTPUT}" ]]; then
		{{SUCCESS}} "${OUTPUT}"
	else
		{{ERROR}} "{{CONTAINER_TAG}} is not running..."
	fi

# Check if the Docker container has exited
[group("Docker"), no-exit-message]
d-is-exited:
	#!/usr/bin/env bash
	set -euo pipefail
	OUTPUT=$(docker ps -a --filter 'name=^/{{CONTAINER_TAG}}$$' --filter 'status=exited' --format '{{ "{{.Names}}: {{.Status}}" }}')
	if [[ -n "${OUTPUT}" ]]; then
		{{SUCCESS}} "${OUTPUT}"
	else
		{{ERROR}} "{{CONTAINER_TAG}} is not in an exited state..."
	fi



# Copy SSH keys to container for SSH key-based login
[group("Docker"), no-exit-message]
@d-setup-ssh-login: d-is-running
	@docker cp ~/.ssh/*.pub {{CONTAINER_TAG}}:/tmp/
	@docker exec {{CONTAINER_TAG}} \
		sh -c 'cat /tmp/*.pub >> {{CONTAINER_HOME}}/.ssh/authorized_keys'
	{{SUCCESS}} "SSH public key setup complete."

# Sets up a clean container for testing data transfer, including adding a debug binary to /usr/bin
[group("Docker"), no-exit-message]
d-setup-for-transfer-tests: \
	d-stop \
	d-build \
	d-run \
	d-setup-ssh-login \
	d-clean-tmp \
	build-x86_64-linux-musl \
	(d-cp-to-container x86_64_LINUX_MUSL_DEBUG_BIN "/usr/bin/")

# Build the docker image if it isn't in the image list ('docker images')
[group("Docker"), no-exit-message]
d-build-if-not-built:
	#!/usr/bin/env bash
	if ! just d-check-image-built; then
		just d-build
	fi

# Check if the Docker image exists (has already been built)
[group("Docker"), no-exit-message]
d-check-image-built:
	#!/usr/bin/env bash
	set -euo pipefail
	if docker images {{CONTAINER_TAG}} | grep -q {{CONTAINER_TAG}}; then
		{{SUCCESS}} "Image '{{CONTAINER_TAG}}' exists"
	else
		{{ERROR}} "Image '{{CONTAINER_TAG}}' does not exist"
	fi


# Clear /tmp directory in the Docker container
[group("Docker"), no-exit-message]
d-clean-tmp:
	docker exec {{CONTAINER_TAG}} sh -c 'rm -rf /tmp/*'

[group("Docker"), no-exit-message]
d-cp-to-container FROM TO:
	docker cp {{FROM}} {{CONTAINER_TAG}}:{{TO}}

# Attach interactively to the running Docker container
[group("Docker"), no-exit-message]
d-attach:
	docker exec -it -u {{CONTAINER_USERNAME}} {{CONTAINER_TAG}} /bin/bash

# Run the Docker container with a command in detached mode
[group("Docker"), no-exit-message]
d-run-cmd-detached CMD:
    @echo "Running Docker container '{{CONTAINER_TAG}}' with command: '{{CMD}}'..."
    docker run --detach \
        --publish 127.0.0.1:{{CONTAINER_SSH_PORT}}:{{CONTAINER_SSH_PORT}} \
        --publish 127.0.0.1:{{CONTAINER_QFT_LISTEN_PORT}}:{{CONTAINER_QFT_LISTEN_PORT}} \
        --name {{CONTAINER_TAG}} {{CONTAINER_TAG}} \
        /bin/sh -c '{{CMD}}; echo $? >> {{CONTAINER_HOME}}/command.exit'
    @echo "Container '{{CONTAINER_TAG}}' started with command: '{{CMD}}'."

# Run attach to the running container and execute a command in the background, append the exit code to command.exit
[group("Docker"), no-exit-message]
d-attach-run-bg-cmd CMD:
	docker exec --detach {{CONTAINER_TAG}} /bin/sh -c "(cd {{CONTAINER_HOME}} && nohup {{CMD}} > /dev/null 2>&1 && echo $? >> command.exit &)"
