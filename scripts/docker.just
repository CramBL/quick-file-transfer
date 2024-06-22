# Default image name
CONTAINER_TAG := "alpine-qft-ssh"
CONTAINER_USERNAME := "userfoo"
CONTAINER_PASSWORD := "password"
CONTAINER_HOME := "/home/" + CONTAINER_USERNAME

CONTAINER_SSH_PORT := "54320"

# Build the Docker image
[group("Docker"), no-exit-message]
d-build:
    docker build -t {{CONTAINER_TAG}} ./docker

# Run the Docker container
[group("Docker"), no-exit-message]
d-run:
    docker run --detach --publish {{CONTAINER_SSH_PORT}}:{{CONTAINER_SSH_PORT}} --name {{CONTAINER_TAG}} {{CONTAINER_TAG}}

# Stop and remove the Docker container
[group("Docker"), no-exit-message]
d-stop:
    docker stop {{CONTAINER_TAG}}
    docker rm {{CONTAINER_TAG}}

# SSH into the running container
[group("Docker"), no-exit-message]
d-ssh:
    ssh -p {{CONTAINER_SSH_PORT}} userfoo@localhost

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
