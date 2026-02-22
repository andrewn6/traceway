.PHONY: help dev dev-daemon dev-ui dev-www dev-ingest \
       build build-daemon build-cloud build-ui build-www \
       run run-cloud run-fg \
       check check-daemon check-ui check-www lint fmt \
       sync-openapi docker-build docker-run \
       up down logs ps \
       install-ui install-www clean

# ── Config ───────────────────────────────────────────────────────────────────
DAEMON_PKG   = daemon
API_PORT     = 3000
UI_PORT      = 5173
WWW_PORT     = 5174

# ── Help ─────────────────────────────────────────────────────────────────────
help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

# ── Development ──────────────────────────────────────────────────────────────
dev: ## Run daemon (foreground) + UI dev server
	@echo "Starting daemon + UI..."
	@trap 'kill 0' EXIT; \
		cargo run -p $(DAEMON_PKG) -- --foreground & \
		cd ui && npm run dev -- --port $(UI_PORT) & \
		wait

dev-daemon: ## Run daemon in foreground (SQLite, default)
	cargo run -p $(DAEMON_PKG) -- --foreground

dev-ui: ## Run UI dev server
	cd ui && npm run dev -- --port $(UI_PORT)

dev-www: ## Run landing page dev server
	cd www && npm run dev -- --port $(WWW_PORT)

dev-ingest: ## Run daemon with synthetic trace ingest (testing)
	cargo run -p $(DAEMON_PKG) -- --foreground --dev-ingest --dev-ingest-interval 3

dev-all: ## Run daemon + UI + landing page
	@echo "Starting daemon + UI + www..."
	@trap 'kill 0' EXIT; \
		cargo run -p $(DAEMON_PKG) -- --foreground & \
		cd ui && npm run dev -- --port $(UI_PORT) & \
		cd www && npm run dev -- --port $(WWW_PORT) & \
		wait

# ── Build ────────────────────────────────────────────────────────────────────
build: build-daemon build-ui ## Build daemon + UI

build-daemon: ## Build daemon (local/SQLite mode)
	cargo build -p $(DAEMON_PKG)

build-release: ## Build daemon release binary
	cargo build -p $(DAEMON_PKG) --release

build-cloud: ## Build daemon with cloud features
	cargo build -p $(DAEMON_PKG) --features cloud

build-cloud-release: ## Build daemon release with cloud features
	cargo build -p $(DAEMON_PKG) --release --features cloud

build-ui: ## Build UI for production
	cd ui && npm run build

build-www: ## Build landing page for production
	cd www && npm run build

# ── Run ──────────────────────────────────────────────────────────────────────
run: ## Run daemon (daemonizes to background)
	cargo run -p $(DAEMON_PKG) -- --daemon

run-fg: ## Run daemon in foreground
	cargo run -p $(DAEMON_PKG) -- --foreground

run-cloud: ## Run daemon in cloud mode (reads .env)
	cargo run -p $(DAEMON_PKG) --features cloud -- --foreground --cloud

# ── Check / Lint ─────────────────────────────────────────────────────────────
check: check-daemon check-ui ## Run all checks

check-daemon: ## Cargo check the daemon
	cargo check -p $(DAEMON_PKG)

check-all-crates: ## Cargo check entire workspace
	cargo check --workspace

check-ui: ## Svelte check the UI
	cd ui && npm run check

check-www: ## Svelte check the landing page
	cd www && npm run check

lint: ## Run clippy on workspace
	cargo clippy --workspace -- -W clippy::all

fmt: ## Format Rust code
	cargo fmt --all

fmt-check: ## Check Rust formatting
	cargo fmt --all --check

# ── OpenAPI ──────────────────────────────────────────────────────────────────
sync-openapi: ## Regenerate openapi.json + TS types from running daemon
	./scripts/sync-openapi.sh

sync-openapi-check: ## Check if openapi files are up to date
	./scripts/sync-openapi.sh --check

generate-types: ## Generate TS types from openapi.json (no daemon needed)
	cd ui && npm run generate-types:file

# ── Docker ───────────────────────────────────────────────────────────────────
docker-build: ## Build Docker image
	docker build -t traceway .

docker-run: ## Run Docker container (standalone, SQLite)
	docker run --rm -p 3000:3000 --env-file .env traceway

# ── Docker Compose (cloud stack) ─────────────────────────────────────────────
up: ## Start cloud stack (daemon + Postgres)
	docker compose up -d

up-build: ## Rebuild and start cloud stack
	docker compose up -d --build

down: ## Stop cloud stack
	docker compose down

down-clean: ## Stop cloud stack and remove volumes
	docker compose down -v

logs: ## Tail cloud stack logs
	docker compose logs -f

logs-daemon: ## Tail daemon logs only
	docker compose logs -f traceway

ps: ## Show running containers
	docker compose ps

# ── Install deps ─────────────────────────────────────────────────────────────
install: install-ui install-www ## Install all JS dependencies

install-ui: ## Install UI dependencies
	cd ui && npm install

install-www: ## Install landing page dependencies
	cd www && npm install

# ── Clean ────────────────────────────────────────────────────────────────────
clean: ## Clean all build artifacts
	cargo clean
	rm -rf ui/build ui/.svelte-kit
	rm -rf www/build www/.svelte-kit
