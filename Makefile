# fsoc - build the game world and run the TUI, per campaign.
#
# Pick a campaign with CAMPAIGN=<name> (default: fsociety). The world's
# user prefix / pass dir / container / image are read from that campaign's
# campaign.toml, so both skins share one Dockerfile and one set of scripts.
CAMPAIGN ?= fsociety
CDIR      = campaigns/$(CAMPAIGN)

val = $(shell sed -n 's/^$(1) *= *"\(.*\)"/\1/p' $(CDIR)/campaign.toml)
num = $(shell sed -n 's/^$(1) *= *\([0-9]*\)/\1/p' $(CDIR)/campaign.toml)

PREFIX    = $(call val,user_prefix)
PASS_DIR  = $(call val,pass_dir)
CONTAINER = $(call val,container)
IMAGE     = $(call val,image)
HOSTN     = $(call val,hostname)
MAX_LEVEL = $(call num,max_level)
PORT     ?= 2220

.PHONY: help world-build world-up world-down world-logs check play build fmt clean

help:
	@echo "fsoc  (CAMPAIGN=$(CAMPAIGN))"
	@echo "  make world-up   [CAMPAIGN=bandit|fsociety]   build + run the world"
	@echo "  make world-down [CAMPAIGN=...]               stop + remove the world"
	@echo "  make check      [CAMPAIGN=...]               headless doctor"
	@echo "  make play       [CAMPAIGN=...]               launch the TUI"
	@echo ""
	@echo "  resolved: prefix=$(PREFIX) pass_dir=$(PASS_DIR) container=$(CONTAINER) image=$(IMAGE) max=$(MAX_LEVEL)"

world-build:
	docker build \
	  --build-arg USER_PREFIX=$(PREFIX) \
	  --build-arg PASS_DIR=$(PASS_DIR) \
	  --build-arg MAX_LEVEL=$(MAX_LEVEL) \
	  -t $(IMAGE) world

world-up: world-build
	-docker rm -f $(CONTAINER) 2>/dev/null
	docker run -d --name $(CONTAINER) $(if $(HOSTN),--hostname $(HOSTN),) -p $(PORT):2220 $(IMAGE)
	@echo "World up ($(CAMPAIGN)). ssh $(PREFIX)0@localhost -p $(PORT)  (password: $(PREFIX)0)"

world-down:
	-docker rm -f $(CONTAINER)

world-logs:
	docker logs -f $(CONTAINER)

build:
	cd tui && cargo build --release

check: build
	./tui/target/release/fsoc --campaign $(CAMPAIGN) --check

play: build
	./tui/target/release/fsoc --campaign $(CAMPAIGN)

fmt:
	cd tui && cargo fmt

clean:
	cd tui && cargo clean

# --- world network (the interconnected world) ---
NET = world/net/compose.yml
MACHINE_IMAGE = fsoc-machine:slice

.PHONY: net-build net-up net-down net-play net-ps

net-build:
	docker build -t $(MACHINE_IMAGE) -f world/machine/Dockerfile world

net-up: net-build
	docker compose -f $(NET) up -d
	@echo "world up. enter: make net-play  (start on workstation, then: ssh relay / ssh archive)"

net-down:
	docker compose -f $(NET) down

net-ps:
	docker compose -f $(NET) ps

net-play:
	docker exec -it fsoc-workstation su - operator
