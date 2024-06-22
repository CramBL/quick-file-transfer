# Default image name
CONTAINER_TAG := "alpine-qft-ssh"
CONTAINER_USERNAME := "userfoo"
CONTAINER_PASSWORD := "password"
CONTAINER_HOME := "/home/" + CONTAINER_USERNAME

CONTAINER_QFT_LISTEN_PORT := "12993"
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
    docker stop {{CONTAINER_TAG}}
    docker rm {{CONTAINER_TAG}}

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
d-status:
	#!/usr/bin/env bash
	set -euo pipefail
	OUTPUT=$(docker ps --filter 'name=^/{{CONTAINER_TAG}}$$' --format '{{ "{{.Names}}: {{.Status}}" }}')
	if [[ -n "${OUTPUT}" ]]; then
		{{SUCCESS}} "${OUTPUT}"
	else
		{{ERROR}} "{{CONTAINER_TAG}} is not running..."
	fi

# Copy SSH keys to container for SSH key-based login
[group("Docker"), no-exit-message]
@d-setup-ssh-login: d-status
	@docker cp ~/.ssh/*.pub {{CONTAINER_TAG}}:/tmp/
	@docker exec {{CONTAINER_TAG}} \
		sh -c 'cat /tmp/*.pub >> {{CONTAINER_HOME}}/.ssh/authorized_keys'
	{{SUCCESS}} "SSH public key setup complete."
