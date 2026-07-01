# syntax=docker/dockerfile:1.7

# Builds the Vite SPA and serves it on :80 inside the docker network.
# Public access + security headers + /api proxying live in the edge `nginx/`
# service.

# ─── Stage 1: build ──────────────────────────────────────────────────
FROM node:22-alpine AS builder
WORKDIR /app

# Corporate CA injection (for SSL-inspecting proxies). Empty file = no-op.
COPY corporate-ca.pem /usr/local/share/ca-certificates/corporate-ca.crt
RUN apk add --no-cache ca-certificates \
 && update-ca-certificates
ENV NODE_EXTRA_CA_CERTS=/etc/ssl/certs/ca-certificates.crt
RUN npm config set cafile /etc/ssl/certs/ca-certificates.crt

# Deps layer — cached unless package.json / lock changes.
COPY package.json package-lock.json* ./
RUN if [ -f package-lock.json ]; then npm ci; else npm install; fi

# Source + Vite production build → /app/dist
COPY . .
RUN npm run build

# ─── Stage 2: nginx static runtime ───────────────────────────────────
# `nginx-unprivileged` is the same nginx image, repacked to run entirely
# as a non-root user (uid 101) and listen on :8080 instead of :80.
# Defence in depth: a container escape via nginx no longer lands as root.
FROM nginxinc/nginx-unprivileged:1.31-alpine AS runtime
COPY --from=builder /app/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 8080

# Use 127.0.0.1 instead of `localhost` — Alpine's wget resolves localhost
# to ::1 first, but nginx-unprivileged only binds IPv4 by default.
HEALTHCHECK --interval=10s --timeout=3s --start-period=5s \
  CMD wget -q --spider http://127.0.0.1:8080/healthz || exit 1
