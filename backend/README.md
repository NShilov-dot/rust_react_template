# Backend — Rust + Axum + Postgres + Redis

Clean-architecture Rust backend with email/password auth, argon2 hashing and
rotating JWT/refresh tokens. Dependencies only point inward.

```
crates/
├── domain          # Pure business types (User, Email, Password, PasswordHash, errors)
├── application     # Use cases + ports (UserRepository, CacheStore, PasswordHasher, SessionManager)
├── infrastructure  # Adapters: sqlx Postgres repo, Redis cache, Argon2Hasher, RedisJwtSessions
└── api             # axum router, handlers, AuthUser extractor, main.rs
```

Dependency direction: `api → infrastructure → application → domain`.

## Stack

- [axum 0.8](https://docs.rs/axum) — HTTP framework
- [sqlx 0.8](https://docs.rs/sqlx) — async Postgres with runtime-checked queries
- [redis 0.27](https://docs.rs/redis) — async Redis with ConnectionManager
- [argon2 0.5](https://docs.rs/argon2) — password hashing (argon2id), run on `spawn_blocking`
- [jsonwebtoken 9](https://docs.rs/jsonwebtoken) — JWT HS256 for access tokens
- [tokio](https://tokio.rs/) — async runtime
- [tracing](https://docs.rs/tracing) — structured logs

## Prerequisites

- Docker (compose plugin) — sufficient for the all-in-Docker path
- Rust 1.88+ only if you want to run the backend on the host
  (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)

## Run — everything in Docker (recommended)

Compose and Makefile live at the **repo root**. Run from there:

```bash
cd ..
cp .env.example .env
# Replace JWT_SECRET with real entropy:
#   openssl rand -base64 48
make up           # or: docker compose up --build
```

This builds backend + frontend + nginx images and starts the full stack:

| URL                         | Service             |
|-----------------------------|---------------------|
| `http://localhost:5173`     | Edge nginx → `/` proxies to `frontend:80` (SPA static), `/api/*` to `backend:8080` |
| `http://localhost:8080`     | Backend REST API (also exposed for direct curl)        |
| `localhost:5432`            | Postgres                                                |
| `localhost:6379`            | Redis                                                   |

Compose waits for Postgres and Redis healthchecks before launching the
backend. Nginx starts in parallel with the backend; nginx will 502
on `/api/*` until the backend is up (usually a few seconds).

## Run — backend on host, deps in Docker

From repo root:

```bash
make env          # creates root .env (compose) + backend/.env (cargo)
make deps-up      # postgres + redis only
make run          # cargo run -p api on the host
```

Or by hand:

```bash
cp .env.example .env                  # root .env for compose substitution
cp backend/.env.example backend/.env  # backend/.env with DATABASE_URL=localhost
docker compose up -d postgres redis
cargo --manifest-path backend/Cargo.toml run -p api
```

## Docker image

Multi-stage Dockerfile (`Dockerfile`):

1. **chef** — `rust:1.90-slim-bookworm` with `cargo-chef` installed.
2. **planner** — extracts a dependency recipe (`recipe.json`).
3. **builder** — `cargo chef cook --release` builds *only* the deps as a
   cached layer; then the real source is copied and `cargo build --release`
   compiles the app. Source edits don't bust the dep cache.
4. **runtime** — `gcr.io/distroless/cc-debian12:nonroot` (~25 MB,
   glibc + ca-certs, no shell, no package manager, runs as uid 65532).
   Only the stripped binary is copied in. Migrations are embedded into the
   binary at compile time via `sqlx::migrate!`, so the runtime image doesn't
   need the `migrations/` folder.

Build standalone:

```bash
docker build -t rust-react-backend:dev .
```

## Endpoints

### Public

| Method | Path              | Body                                    | Description                              |
|--------|-------------------|------------------------------------------|------------------------------------------|
| GET    | `/health`         |                                          | Liveness                                 |
| POST   | `/auth/register`  | `{ email, name, password }`             | Create user + Set-Cookie refresh + access|
| POST   | `/auth/login`     | `{ email, password }`                   | Verify creds + Set-Cookie refresh + access|
| POST   | `/auth/refresh`   | _(cookie)_                              | Rotate refresh in cookie, return new access |
| POST   | `/auth/logout`    | _(cookie)_                              | Revoke + clear cookie                    |

### Protected — require `Authorization: Bearer <access_token>`

| Method | Path              | Body                  | Description                       |
|--------|-------------------|-----------------------|-----------------------------------|
| GET    | `/auth/me`        |                       | Current user (Redis-cached)       |
| GET    | `/users`          | `?limit=20&offset=0`  | List users                        |
| GET    | `/users/{id}`     |                       | Get user by id (Redis-cached)     |

### Response shape

**login / register** — refresh token lives in HttpOnly cookie, NOT in JSON:

```json
{
  "user": { "id": "...", "email": "...", "name": "...", "created_at": "...", "updated_at": "..." },
  "access_token": "eyJhbGc...",
  "access_expires_at": "2026-06-11T15:30:00Z"
}
```

Plus this Set-Cookie header:

```
Set-Cookie: refresh_token=f8c2…3a.dQw4w9WgX…; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age=2592000
```

**refresh** — same Set-Cookie + body `{ access_token, access_expires_at }`.

**logout** — `Set-Cookie: refresh_token=; Max-Age=0` + `204 No Content`.

## Quick smoke test

```bash
curl -s -X POST localhost:8080/auth/register \
  -H 'content-type: application/json' \
  -d '{"email":"a@b.co","name":"Alice","password":"correcthorsebatterystaple"}' | jq

ACCESS=$(...)  # extract from response
curl -s localhost:8080/auth/me -H "authorization: Bearer $ACCESS" | jq
```

## Auth design

### Access tokens — stateless JWT

- HS256, 15-minute TTL by default.
- Claims: `sub` (user_id), `iss` (configurable), `iat`, `exp`.
- Verified inline in the `AuthUser` extractor — no Redis hit per request.

### Refresh tokens — opaque, stored in Redis, rotated

- Format: `{family_id_hex}.{32_random_bytes_base64url}` — encoding the family
  in the token lets us locate the right Redis keys without an extra lookup.
- Single-use: every `/auth/refresh` issues a new pair and invalidates the old.
- Default TTL 30 days.

### Reuse detection (token theft mitigation)

Each login creates a "family". Refresh tokens within a family form a chain:
the current one is tracked in `family:{id}:current`. If a token from earlier
in the chain is presented (i.e., someone is re-using a rotated token), the
entire family is revoked atomically — the legitimate session is killed and
the attacker's stolen token stops working.

The rotation runs as a single Lua script in Redis, so the
check-and-swap can't race.

```
KEYS = [refresh:{old}, family:{f}:current, family:{f}:revoked, refresh:{new}]
ARGV = [old_token, new_token, ttl_secs]
```

### Logout

`POST /auth/logout` deletes the current refresh token and family marker.
Access tokens remain valid until their (short) expiry — they're stateless
by design.

## Test

```bash
cargo test --workspace
```

## Why this layout

- **Domain** has no I/O. `Email`, `User::new`, `Password::parse` enforce
  invariants at construction.
- **Application** defines `UserRepository`, `CacheStore`, `PasswordHasher`,
  `SessionManager` traits. Use cases consume `Arc<dyn Trait>` — trivially
  mockable. The use cases don't know that auth uses argon2 + JWT + Redis;
  swap any of those in infrastructure without touching the application layer.
- **Infrastructure** wires the traits to real adapters.
- **Api** translates HTTP → use-case calls. The `AuthUser` extractor is the
  only place HTTP knows about auth at all.

## Configuration

| Var                  | Default                          | Notes                          |
|----------------------|----------------------------------|--------------------------------|
| `DATABASE_URL`       | —                                | required                       |
| `REDIS_URL`          | —                                | required                       |
| `BIND_ADDR`          | `0.0.0.0:8080`                   |                                |
| `DB_MAX_CONNECTIONS` | `10`                             |                                |
| `RUST_LOG`           | `info,api=debug,tower_http=info` |                                |
| `JWT_SECRET`         | —                                | required, ≥ 32 bytes           |
| `JWT_ISSUER`         | `rust-react-api`                 |                                |
| `ACCESS_TTL_SECS`    | `900` (15 min)                   | keep short                     |
| `REFRESH_TTL_SECS`   | `2592000` (30 days)              | sliding window via rotation    |

## Rate limits (per IP, token-bucket via `tower-governor`)

| Endpoint          | Sustained rate | Burst | On 429 |
|-------------------|----------------|-------|--------|
| `POST /auth/login`    | 5 / min  | 5  | `Retry-After` header |
| `POST /auth/register` | 3 / min  | 3  | `Retry-After` header |
| `POST /auth/refresh`  | 30 / min | 30 | `Retry-After` header |

`SmartIpKeyExtractor` reads `Forwarded` / `X-Forwarded-For` first (nginx sets
them), and falls back to peer IP via `ConnectInfo` for direct curl tests.

## Things not in this scaffold (good follow-ups)

- Email verification, password reset.
- Per-user "revoke all sessions" — index families by `user_id` in Redis.
- Replace runtime sqlx queries with compile-time `query!` once you wire up
  `cargo sqlx prepare` in CI.
