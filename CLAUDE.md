# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repo layout

Three concerns in one repo, orchestrated by the root `docker-compose.yml` + `Makefile`:

- `backend/` — Rust workspace (Axum + Postgres + Redis), auth API. Has its own `Dockerfile`.
- `frontend/app/` — Vite + React SPA. Its `Dockerfile` builds the SPA and serves the static files via an internal nginx on port 80 (not published externally).
- `nginx/` — **edge reverse proxy**. Public-facing on `:5173`. Routes `/api/*` to `backend:8080` and everything else to the internal `frontend:80` upstream. Sets CSP / security headers here.

Root files: `Makefile`, `docker-compose.yml`, `.env.example`. **All `make`/`docker compose` commands run from the repo root.**

The frontend has **no `package.json` in `frontend/`** — only in `frontend/app/`. All npm commands run from `frontend/app/`.

## Common commands

### Full stack (run from repo root)

```bash
make up           # build + start everything (postgres, redis, backend, frontend, nginx), follow logs
make up-d         # same, detached
make down         # stop (keep volumes)
make nuke         # stop + wipe Postgres/Redis volumes
make rebuild      # rebuild backend image + restart
make rebuild-frontend  # rebuild SPA (frontend container) + restart
make rebuild-nginx     # rebuild edge nginx + restart
make rebuild-clean # rebuild backend with --no-cache (after Rust/dep bumps)
make logs-backend / make logs-frontend / make logs-nginx
make psql / make redis-cli
make health        # curl :5173/api/health (through edge)
make health-direct # curl :8080/health (direct)
```

### Observability overlay (optional)

```bash
make obs-up-d      # full stack + grafana/prometheus/tempo/loki/promtail/otel-collector
make obs-grafana   # opens http://localhost:3000
make obs-logs      # tail logs from the observability services only
make obs-down      # stop everything (keeps volumes)
make obs-nuke      # stop AND wipe Prom/Tempo/Loki/Grafana data
```

`make up` requires **two** .env files (`make env` creates both if missing):
- root `.env` — read by `docker compose` for variable substitution (`JWT_SECRET` etc.).
- `backend/.env` — only used by `cargo run` on the host; has `DATABASE_URL=localhost`.

Replace `JWT_SECRET` in the root `.env` with real entropy (`openssl rand -base64 48`) before exposing the stack.

### Backend on host (run from repo root — Makefile uses `cargo --manifest-path backend/Cargo.toml`)

```bash
make deps-up      # bring up postgres + redis in docker
make run          # cargo run -p api  (needs deps-up first)
make build        # cargo build --release -p api
make test         # cargo test --workspace
make check        # cargo check --workspace --all-targets
make lint         # fmt-check + clippy -D warnings  (use this in CI)
make fmt          # cargo fmt --all
make clippy       # cargo clippy --workspace --all-targets -- -D warnings
make migration NAME=add_foo  # creates backend/migrations/<timestamp>_add_foo.sql
```

Single test (direct, from root): `cargo --manifest-path backend/Cargo.toml test -p <crate> <test_name>` — or just `cd backend && cargo test -p domain password::tests::rejects_short`.

### Frontend (run from `frontend/app/`)

```bash
npm install
npm run dev       # http://localhost:5173, proxies /api → :8080
npm run build     # tsc --noEmit + vite build → dist/
npm run typecheck # strict TS, no emit
npm run lint      # eslint, warnings → errors
npm run test      # vitest run
npm run size      # size-limit gate (after build)
```

Single test: `npx vitest run <path>` (vitest config in `vitest.config.ts`).

## Architecture

### Backend — clean architecture, dependencies point inward

```
crates/
├── domain         # Pure types (User, Email, Password, PasswordHash, errors). No I/O.
├── application    # Use cases + ports (UserRepository, CacheStore, PasswordHasher, SessionManager).
├── infrastructure # Adapters: sqlx Postgres repo, Redis cache, Argon2Hasher, RedisJwtSessions.
└── api            # axum router, handlers, AuthUser extractor, main.rs (wiring).
```

Dependency direction: `api → infrastructure → application → domain`. **Never reverse this.** Domain types enforce invariants at construction (`Email::parse`, `Password::parse`, `User::new`). Use cases take `Arc<dyn Trait>` — easy to mock; swap argon2/JWT/Redis without touching application layer.

Migrations are embedded into the binary via `sqlx::migrate!("../../migrations")` in `api/src/main.rs`, so the distroless runtime image carries no migration files.

### Backend — auth (the non-obvious bits)

- **Access tokens** are stateless JWT (HS256, 15 min default). Verified inline in the `AuthUser` extractor — **no Redis hit per request**.
- **Refresh tokens** are opaque, format `{family_id_hex}.{32_random_bytes_b64url}` — the family is encoded in the token so rotation finds the right Redis keys without an index lookup.
- **Reuse detection**: each login creates a "family". Refresh tokens chain inside a family; presenting an earlier (already-rotated) token revokes the whole family atomically — kills the legitimate session AND the attacker. Check-and-swap runs as a single Lua script in Redis to prevent races.
- Refresh token goes in an **HttpOnly cookie** (Set-Cookie on register/login/refresh), NOT in the JSON body. Only the access token is in JSON.
- Rate limiting: `tower_governor` per-IP, with `SmartIpKeyExtractor` reading `Forwarded`/`X-Forwarded-For` (nginx) and falling back to `ConnectInfo` (direct curl). That's why `main.rs` uses `into_make_service_with_connect_info::<SocketAddr>`.

### Google OAuth — design notes

- **Server-side Authorization Code + PKCE + state**. Frontend never sees `client_secret`; the SPA just navigates to `/api/auth/google/start`.
- **`GoogleAuth` use case** (`application/auth/google.rs`) owns the linking policy. Three branches: known `google_id` → log in; new `google_id` + verified-email match → auto-link existing user; otherwise → create OAuth-only user (NULL `password_hash`). **Auto-link only happens when Google asserts `email_verified=true`** — otherwise we refuse, since unverified linking is the classic account-takeover vector.
- **State + PKCE storage** uses the existing `CacheStore` (Redis), key `oauth:google:state:<csrf>`, TTL 5 min, deleted after first use (one-shot replay protection).
- **Schema**: `password_hash` is now NULLable. New OAuth users have `password_hash IS NULL` + a `google_id`. `find_for_login` filters `WHERE password_hash IS NOT NULL`, so password-login for an OAuth-only account fails as `InvalidCredentials` (semantically correct, doesn't leak account existence).
- **Feature toggle**: if `GOOGLE_CLIENT_ID` or `GOOGLE_CLIENT_SECRET` is missing, `GoogleAuth` is `None` in `AppState` and the routes respond `503`. No env, no feature, no compile-time on/off switch.
- **Callback redirect**: backend sets the refresh cookie and 302s to `OAUTH_POST_LOGIN_REDIRECT` (default `/dashboard`). The SPA's `SessionBootstrap` runs on the new page load, the cookie is present, silent `/auth/refresh` populates the in-memory access token. On failure the redirect target is `OAUTH_ERROR_REDIRECT` (default `/login`) with `?oauth_error=<code>` — the `useOAuthError` hook on the SPA reads, displays, then strips it.
- **Error codes** (kept stable, see `api/handlers/google.rs` `classify()`): `denied`, `expired`, `unverified`, `network`, `internal`, `bad_request`. They're deliberately vague — never reveal "email exists" or "google already linked" via the redirect.

### Frontend — XSS-resistant auth pattern

The auth flow in `src/lib/api.ts` + `src/lib/auth-store.ts` enforces a specific XSS-resistant pattern. **Do not break it casually:**

1. Access token lives **only in memory** (Zustand without `persist`). XSS can read it for the lifetime of the tab and max 15 min.
2. Refresh token is the HttpOnly cookie (JS can't read it). All fetch calls use `credentials: 'include'`.
3. `SessionBootstrap` silently calls `/auth/refresh` on app mount — that's how "F5 doesn't log you out" works with in-memory tokens.
4. `/auth/refresh` is **single-flight within a tab AND serialized across tabs** via `navigator.locks.request('auth-refresh', …)`. Without this, two tabs racing would trip the backend's reuse-detection and kill the session.
5. 401 responses auto-retry once with a fresh token; if refresh fails the store clears and `ProtectedRoute` redirects to `/login`.

### Frontend stack notes

- **Vite 5** + React 18 + TypeScript (strict, `noUncheckedIndexedAccess`).
- **TanStack Query v5** owns server state; **Zustand** owns auth/UI state. Don't reach for Redux or React Context as a state container — both are explicit anti-patterns here.
- Routes are **lazy-loaded** per route (code splitting). `npm run size` enforces ≤ 200 KB gzip initial / ≤ 80 KB gzip per-route chunk via `size-limit` (config in `package.json`).
- `vite.config.ts` proxies `/api/*` → `http://localhost:8080/*` in dev. In production, nginx (`frontend/app/nginx.conf`) does the same proxying — so the frontend code never knows the absolute backend URL.

### Docker

Five services in `docker-compose.yml`: `postgres`, `redis`, `backend`, `frontend`, `nginx`.

- Only `nginx` (`:5173`) and `backend` (`:8080`, kept for direct curl) are published. `frontend:80` is internal-only and is fronted by nginx.
- Backend image is **distroless** (`gcr.io/distroless/cc-debian12:nonroot`) — no shell, no package manager. Compose can't run a healthcheck against it; nginx's `depends_on: backend` is startup-order only and nginx will 502 on `/api/*` for a few seconds during boot.
- Backend Dockerfile uses **`cargo-chef`** to cache dependency builds: source edits don't bust the dep layer.
- Edge `nginx/` is a thin proxy — its Dockerfile just bakes in `nginx.conf` on `nginx:alpine`. All security headers (CSP, X-Frame-Options, etc.) live in `nginx/nginx.conf`, NOT in `frontend/app/nginx.conf` (which only serves static + SPA fallback).
- Two `upstream` blocks in `nginx/nginx.conf` reference services by their compose names (`frontend:80`, `backend:8080`) — Docker's embedded DNS resolves these.

## Configuration

- Root `.env.example` — minimal vars used by `docker compose` (`JWT_SECRET`, `JWT_ISSUER`, TTLs, `RUST_LOG`). DB / Redis URLs are set per-service in the compose file.
- `backend/.env.example` — full set for running the backend on the host (`DATABASE_URL=localhost`, etc.).

`JWT_SECRET` must be ≥ 32 bytes; the compose file fails fast (`${JWT_SECRET:?…}`) if missing. `ACCESS_TTL_SECS` should stay short (default 900) since access tokens are stateless and only revoked by expiry.

## Endpoints (backend)

Public: `GET /health`, `GET /metrics`, `POST /auth/register`, `POST /auth/login`, `POST /auth/refresh`, `POST /auth/logout`.
Protected (`Authorization: Bearer <access>`): `GET /auth/me`, `GET /users?limit=&offset=`, `GET /users/{id}`.

`/auth/me` and `/users/{id}` are Redis-cached.

`/metrics` exposes Prometheus format — RED metrics (`axum_http_requests_*`) and business counters (`auth_attempts_total{endpoint,outcome}`). Internal-only in spirit; nginx happens to pass it through `/api/metrics`, which is fine for local dev. For production you'd want a deny rule in `nginx/nginx.conf` or a separate metrics port.

## Observability

The app is instrumented; the backend stack is an **optional overlay** so day-to-day dev is lean.

- **Three signals on the app side:** Prometheus metrics on `:8080/metrics`, OTLP traces shipped to `OTEL_EXPORTER_OTLP_ENDPOINT` when set, and stdout logs that switch to JSON when `LOG_FORMAT=json`. All three layers wired in `api/src/telemetry.rs` — single `telemetry::init()` call in `main.rs` (plus a `Drop`/`shutdown` on exit for OTLP flushing).
- **Tracing pipeline order:** EnvFilter → stdout (pretty/JSON) → tracing-opentelemetry (OTLP). HTTP RED metrics are auto-emitted by an `axum-prometheus` Tower layer; business counters use the `metrics::counter!` macro (kept low-cardinality — one `endpoint` × one `outcome` label = ≤ 8 series).
- **Observability stack** lives in `observability/` (otel-collector, prometheus, tempo, loki, promtail, grafana — each with its own config file) and is launched via `docker-compose.observability.yml` overlay. Grafana is provisioned with Prom/Tempo/Loki datasources and one starter dashboard (`grafana/dashboards/backend-red.json`).
- **Commands**: `make obs-up-d` (or `obs-up` to follow logs) brings everything up. `make obs-grafana` opens Grafana at `http://localhost:3000` (admin/admin, or anonymous Viewer). `make obs-nuke` wipes Prom/Tempo/Loki/Grafana volumes.
- **Trace ↔ logs correlation**: Grafana's Tempo datasource is wired so that clicking a span jumps to Loki logs for the same `trace_id`. The JSON formatter on the app side includes `trace_id` when a span is active.
