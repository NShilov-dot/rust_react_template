# syntax=docker/dockerfile:1.7

# Edge reverse proxy: terminates the public :5173 port, sets security
# headers, routes `/api/*` to backend:8080 and everything else to the
# internal `frontend` service.

FROM nginx:1.27-alpine
COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80

HEALTHCHECK --interval=10s --timeout=3s --start-period=5s \
  CMD wget -q --spider http://localhost/healthz || exit 1
