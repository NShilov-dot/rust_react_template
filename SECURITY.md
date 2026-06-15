# Security

This is a **template** repository. The notes below describe the security
posture of the stock template and the steps a downstream consumer should
take before exposing it to real users.

## Reporting a vulnerability

If you spot a vulnerability in the template itself, please **do not** open
a public issue. Open a private GitHub Security Advisory or email the
maintainer listed in the repository's `CODEOWNERS`. We aim to respond
within seven days and to ship a fix or mitigation within thirty days for
high-severity issues, per the [responsible disclosure timeline](#disclosure-timeline)
below.

When you fork this template for a real product, **replace this paragraph**
with the contact channel for your own security team.

## Security model at a glance

| Layer | Defence |
|-------|---------|
| Transport (between user ↔ edge nginx) | Out of scope — terminate TLS at your own load balancer / ingress |
| Edge → backend | Internal Docker network only |
| Authentication | JWT access (15 min, in memory) + opaque refresh in `HttpOnly; Secure; SameSite=Strict` cookie |
| Refresh rotation | Single-flight per tab + cross-tab Web Locks; backend reuse-detection revokes the family on replay |
| Password storage | Argon2id, OWASP defaults |
| Authorisation | Repository-level `WHERE owner_id = $auth_user` — IDOR-proof even if a handler forgets the check |
| Rate limiting | `tower_governor`, per-IP; covers auth flows + tasks write path |
| Headers | CSP, X-Frame-Options DENY, X-Content-Type-Options, Referrer-Policy, Permissions-Policy set on the edge nginx |
| Runtime | Backend distroless+nonroot; frontend `nginx-unprivileged` (uid 101) |

## Known issues & accepted risks

The template deliberately carries the following accepted risks. Each one
is either unreachable in the stock code paths or unavoidable given
upstream constraints; review them when you fork.

### `time = "=0.3.36"` pin (RUSTSEC-2026-0009)

CVSS 6.8 (medium). The pinned `time` patch has a stack-exhaustion DoS in
its format-string parser. We hold the pin because `cookie 0.18.1` does
not compile against newer `time` under Rust 1.90's coherence checks.

**Reachability**: not exposed. The vulnerable code paths require a
caller to pass attacker-controlled format strings to `time::parsing`.
The template uses `chrono` for all user-facing datetime parsing;
`cookie` only invokes `time` for standardised RFC 7231 / RFC 850 / asctime
cookie expiration parsing. Neither path is attacker-controlled.

**Plan**: drop the pin once `cookie` ships a fix (track
[rwf2/cookie-rs#207](https://github.com/rwf2/cookie-rs/) or successor)
or once we migrate to a maintained cookie crate.

### `rustls 0.21.12` / `rustls-webpki 0.101.7` (RUSTSEC-2026-0098/0099/0104)

Three medium-severity advisories on certificate validation. Pulled
transitively by `sqlx-runtime-tokio-rustls`. `cargo update` cannot
move this without a `sqlx` minor bump that may not yet exist.

**Reachability**: not exposed in the template. `sqlx` connects to a
Postgres instance over the internal Docker network with TLS disabled —
the vulnerable code paths are never executed. If you fork and enable
Postgres TLS (production!) re-evaluate.

**Plan**: pick up the fix automatically when `sqlx` releases against
`rustls 0.23+`.

### `rsa` Marvin Attack (RUSTSEC-2023-0071)

Pulled by `jsonwebtoken` for RS256 support. The template uses HS256
(symmetric HMAC) for JWT issuance — the RSA code path is never invoked.
No upstream fix exists.

**Plan**: track upstream. Re-evaluate if you switch to RS256 or any
asymmetric JWT algorithm.

### `rustls-pemfile 1.0.4` unmaintained (RUSTSEC-2025-0134)

Warning-only advisory. The crate still works; just no further patches.
Track for replacement when sqlx or reqwest upgrade their PEM parsing.

## Hardening checklist before production

These items are intentionally left "off" or "permissive" in the template
because they break local-only HTTP / single-developer workflows. Turn
them on before exposing to the public internet.

- [ ] **HSTS**: uncomment `Strict-Transport-Security` in `nginx/nginx.conf`. Requires HTTPS termination upstream of the edge nginx (load balancer, k8s ingress, etc.).
- [ ] **CSP `style-src`**: drop `'unsafe-inline'` once Tailwind is rebuilt with nonces or once you switch to a runtime-CSS-free build.
- [ ] **`/metrics` allowlist**: in `nginx/nginx.conf`, replace `deny all` with `allow <your-monitoring-cidr>; deny all;`. Without this, Prometheus scrapes will 403.
- [ ] **JWT secret**: replace the placeholder `JWT_SECRET` in `.env` with `openssl rand -base64 48`. The compose file fails fast if it's empty, but does not check entropy.
- [ ] **Database TLS**: enable `sslmode=verify-full` on the `DATABASE_URL`, pin the server cert. The default compose config connects in plaintext over the Docker network.
- [ ] **Refresh-cookie domain**: set `Domain=` explicitly in `api/handlers/auth.rs::refresh_cookie` if your frontend and backend live on different hosts.
- [ ] **Email enumeration on register**: the stock `POST /auth/register` returns `409 Conflict` for taken emails. If your threat model includes mass enumeration, switch to a "we've sent you an email" flow that always returns `202 Accepted`.
- [ ] **CORS**: the template ships without CORS middleware because the edge nginx serves the SPA and the API from the same origin. Add a `tower-http::cors::CorsLayer` if you split origins.
- [ ] **Audit logging**: write authentication events (login success, login failure, token refresh, logout) to a tamper-evident sink. The template logs them via `tracing` only.
- [ ] **Backup encryption**: Postgres + Redis volumes hold PII and session state. Encrypt at rest before going to prod.
- [ ] **Secret scanning**: enable GitHub secret scanning push protection on your fork — it's an extra layer beyond the `.gitignore` rules.

## Disclosure timeline

For vulnerabilities reported in this template:

- **Day 0**: report received, acknowledgement within 48 h.
- **Day 7**: triage complete, severity assigned.
- **Day 30**: fix or mitigation shipped for critical / high.
- **Day 90**: public advisory + CVE request (where appropriate).

We follow the [coordinated vulnerability disclosure](https://www.first.org/global/sigs/vulnerability-coordination/multiparty/guidelines-v1.1) practice.
