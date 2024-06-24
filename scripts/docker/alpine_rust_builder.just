CONTAINER_ALPINE_BUILDER_TAG := "qft-alpine-builder"


# Build the container that can build a binary for the test container (x86_64-unknown-linux-musl)
[group("Docker"), group("Alpine Builder"), no-exit-message]
d-builder-build:
	docker build --file ./docker/Dockerfile_builder \
		-t {{CONTAINER_ALPINE_BUILDER_TAG}} .

# Build the project for x86_64-unknown-linux-musl via the alpine builder container
[group("Docker"), group("Alpine Builder"), no-exit-message]
d-builder-compile *ARGS="":
	docker run \
		-it \
		--rm \
		--volume $(pwd):/usr/src/qft \
		--name {{CONTAINER_ALPINE_BUILDER_TAG}} {{CONTAINER_ALPINE_BUILDER_TAG}} \
		cargo build --target x86_64-unknown-linux-musl {{ARGS}}
