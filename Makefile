COMPOSE := docker compose

# Cargo commands run against the backend workspace from the repo root.
CARGO := cargo --manifest-path backend/Cargo.toml

.DEFAULT_GOAL := help

# ─── Help ─────────────────────────────────────────────────────────
.PHONY: help
help: ## Show this help
	@awk 'BEGIN {FS = ":.*?## "} \
		/^# ─── / {gsub(/^# ─── | ─.*$$/, ""); printf "\n\033[1m%s\033[0m\n", $$0; next} \
		/^[a-zA-Z_-]+:.*?## / {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

# ─── Setup ────────────────────────────────────────────────────────
.PHONY: env
env: ## Create root .env (for compose) and backend/.env (for `cargo run`) if missing
	@test -f .env || (cp .env.example .env && echo "Created .env from .env.example")
	@test -f backend/.env || (cp backend/.env.example backend/.env && echo "Created backend/.env from backend/.env.example")

# ─── Docker compose ───────────────────────────────────────────────
.PHONY: up
up: env ## Start full stack (build + run, follow logs)
	$(COMPOSE) up --build

.PHONY: up-d
up-d: env ## Start full stack detached
	$(COMPOSE) up -d --build

.PHONY: down
down: env ## Stop stack (keep volumes)
	$(COMPOSE) down

.PHONY: nuke
nuke: env ## Stop stack AND wipe Postgres + Redis volumes
	$(COMPOSE) down -v

.PHONY: ps
ps: env ## Show running services
	$(COMPOSE) ps

.PHONY: logs
logs: env ## Tail logs from all services
	$(COMPOSE) logs -f

.PHONY: logs-backend
logs-backend: env ## Tail logs from backend only
	$(COMPOSE) logs -f backend

.PHONY: logs-frontend
logs-frontend: env ## Tail logs from frontend (SPA static) only
	$(COMPOSE) logs -f frontend

.PHONY: logs-nginx
logs-nginx: env ## Tail logs from nginx (edge) only
	$(COMPOSE) logs -f nginx

.PHONY: restart
restart: env ## Restart backend
	$(COMPOSE) restart backend

.PHONY: restart-frontend
restart-frontend: env ## Restart frontend
	$(COMPOSE) restart frontend

.PHONY: restart-nginx
restart-nginx: env ## Restart nginx
	$(COMPOSE) restart nginx

.PHONY: rebuild
rebuild: env ## Rebuild backend image and restart it
	$(COMPOSE) up -d --build backend

.PHONY: rebuild-frontend
rebuild-frontend: env ## Rebuild frontend image (rebuilds the SPA) and restart it
	$(COMPOSE) up -d --build frontend

.PHONY: rebuild-nginx
rebuild-nginx: env ## Rebuild nginx image and restart it
	$(COMPOSE) up -d --build nginx

.PHONY: rebuild-clean
rebuild-clean: env ## Rebuild backend with --no-cache, then bring the whole stack up (use after Rust/dep bumps)
	$(COMPOSE) build --no-cache backend
	$(COMPOSE) up -d

.PHONY: deps-up
deps-up: env ## Start only Postgres + Redis (for `make run` on host)
	$(COMPOSE) up -d postgres redis

# ─── Database access ──────────────────────────────────────────────
.PHONY: psql
psql: env ## Open a psql shell on the running Postgres
	$(COMPOSE) exec postgres psql -U app -d app

.PHONY: redis-cli
redis-cli: env ## Open redis-cli on the running Redis
	$(COMPOSE) exec redis redis-cli

# ─── Local cargo (operates on backend/ workspace) ─────────────────
.PHONY: run
run: env ## Run the API on the host (needs `make deps-up` first)
	$(CARGO) run -p api

.PHONY: build
build: ## Build the release binary on the host
	$(CARGO) build --release -p api

.PHONY: check
check: ## cargo check across the workspace
	$(CARGO) check --workspace --all-targets

.PHONY: test
test: ## cargo test across the workspace
	$(CARGO) test --workspace

.PHONY: fmt
fmt: ## Format all crates
	$(CARGO) fmt --all

.PHONY: fmt-check
fmt-check: ## Verify formatting without writing
	$(CARGO) fmt --all -- --check

.PHONY: clippy
clippy: ## Lint with clippy (warnings → errors)
	$(CARGO) clippy --workspace --all-targets -- -D warnings

.PHONY: lint
lint: fmt-check clippy ## Run fmt-check + clippy (use this in CI)

.PHONY: clean
clean: ## cargo clean
	$(CARGO) clean

# ─── Migrations ───────────────────────────────────────────────────
.PHONY: migration
migration: ## Create a new timestamped migration file (NAME=add_foo)
	@test -n "$(NAME)" || (echo "usage: make migration NAME=add_foo" && exit 1)
	@TS=$$(date -u +%Y%m%d%H%M%S); \
	 FILE="backend/migrations/$${TS}_$(NAME).sql"; \
	 touch "$$FILE" && echo "Created $$FILE"

# ─── Smoke tests ──────────────────────────────────────────────────
.PHONY: health
health: ## curl /health (via nginx edge)
	@curl -fsS http://localhost:5173/api/health | jq .

.PHONY: health-direct
health-direct: ## curl /health (direct to backend on :8080)
	@curl -fsS http://localhost:8080/health | jq .

.PHONY: smoke-register
smoke-register: ## Register a test user (override: EMAIL=, NAME=, PASSWORD=)
	@curl -fsS -X POST http://localhost:8080/auth/register \
		-H 'content-type: application/json' \
		-d '{"email":"$(or $(EMAIL),smoke@test.io)","name":"$(or $(NAME),Smoke Test)","password":"$(or $(PASSWORD),correcthorsebatterystaple)"}' | jq .

.PHONY: smoke-login
smoke-login: ## Log in as test user (override: EMAIL=, PASSWORD=)
	@curl -fsS -X POST http://localhost:8080/auth/login \
		-H 'content-type: application/json' \
		-d '{"email":"$(or $(EMAIL),smoke@test.io)","password":"$(or $(PASSWORD),correcthorsebatterystaple)"}' | jq .
