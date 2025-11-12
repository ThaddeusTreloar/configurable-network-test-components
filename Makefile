

build-image:
	cargo build --release && podman build . -t configurable-test-api:latest
