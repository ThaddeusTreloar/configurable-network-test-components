

build-image-loadgen:
	podman build . -t configurable-load-generator:latest -f configurable-load-generator.Dockerfile

build-image-api:
	podman build . -t configurable-test-api:latest -f configurable-test-api.Dockerfile
