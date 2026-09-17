# wargamezr — build the game world and run the TUI.
CONTAINER ?= bandit
IMAGE     ?= bandit-world:slice
PORT      ?= 2220
MAX_LEVEL ?= 11

.PHONY: help world-build world-up world-down world-logs check play build fmt clean

help:
	@echo "wargamezr targets:"
	@echo "  make world-build   build the Docker game world (MAX_LEVEL=$(MAX_LEVEL))"
	@echo "  make world-up      run the world container (ssh on port $(PORT))"
	@echo "  make world-down    stop and remove the world container"
	@echo "  make check         headless doctor: verify world + level passwords"
	@echo "  make play          build + launch the TUI"
	@echo "  make world-logs    tail the container logs"

world-build:
	docker build --build-arg MAX_LEVEL=$(MAX_LEVEL) -t $(IMAGE) world

world-up: world-build
	-docker rm -f $(CONTAINER) 2>/dev/null
	docker run -d --name $(CONTAINER) -p $(PORT):2220 $(IMAGE)
	@echo "World up. Try: ssh bandit0@localhost -p $(PORT)  (password: bandit0)"

world-down:
	-docker rm -f $(CONTAINER)

world-logs:
	docker logs -f $(CONTAINER)

build:
	cd tui && cargo build --release

check: build
	./tui/target/release/wargamezr --check

play: build
	./tui/target/release/wargamezr

fmt:
	cd tui && cargo fmt

clean:
	cd tui && cargo clean
