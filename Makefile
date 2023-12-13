DOCKER_BUILD_STATE_FILE := .wontun-remote-docker-build

target/release/wontun:
	cargo build --release

wontun-remote-docker: $(DOCKER_BUILD_STATE_FILE)

$(DOCKER_BUILD_STATE_FILE): Dockerfile scripts/run_docker.sh target/release/wontun
	docker build -t wontun-remote:latest .
	@touch $(DOCKER_BUILD_STATE_FILE)

.PHONY: wontun-remote-docker
