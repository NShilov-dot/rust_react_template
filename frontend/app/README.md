# frontend/app — Vite SPA для Rust бэкенда

Auth-walled SPA, общается с `backend/` (Rust) по REST через `/api` proxy в dev
и через тот же путь в проде.

`Dockerfile` собирает SPA и отдаёт её внутренним nginx на :80 — наружу этот порт
не публикуется. Публичный edge-прокси, security headers и `/api` → backend живут
в корневом сервисе `nginx/` (см. корневой `docker-compose.yml`).

## Стек

- **Vite 5** + React 18 + TypeScript (strict + `noUncheckedIndexedAccess`)
- **React Router 6** — client-side routing, route-level code splitting
- **TanStack Query v5** — server state
- **Zustand** (persist middleware) — auth state (tokens + user)
- **React Hook Form + Zod** — формы и валидация
- **Tailwind CSS** + минимальный shadcn-style UI (Button / Input / Card)
- **Vitest** + Testing Library — unit

## Запуск

```bash
# 1. Поднять зависимости (postgres + redis + backend) из корня
(cd ../.. && make up-d)

# 2. Установить и запустить dev-сборку SPA
npm install
npm run dev    # http://localhost:5173
```

⚠️ Dev-сервер Vite слушает тот же `:5173`, что и edge-nginx из compose. Когда
поднимаешь Vite, либо потуши контейнер nginx (`(cd ../.. && make down)`), либо
поменяй порт в `vite.config.ts`.

Vite dev server проксирует `/api/*` → `http://localhost:8080/*`, так что фронту
не нужно знать про CORS. В проде ту же роль (статика + прокси) выполняют
сервисы `frontend` + `nginx` из корневого compose.

## Скрипты

| Команда | Действие |
|---|---|
| `npm run dev` | dev server с HMR |
| `npm run build` | typecheck + production build в `dist/` |
| `npm run preview` | предпросмотр production-сборки |
| `npm run typecheck` | строгий TS check без билда |
| `npm run lint` | ESLint (warnings → errors) |
| `npm run test` | Vitest run |
| `npm run size` | size-limit гейт по бандлам |

## Auth flow

Реализован против эндпойнтов бэкенда:

| Эндпойнт | Что делает |
|---|---|
| `POST /auth/register` | login и сразу выдаёт пару токенов |
| `POST /auth/login` | то же, для существующих |
| `POST /auth/refresh` | rotation (single-flight, см. ниже) |
| `POST /auth/logout` | revoke refresh + clear store |
| `GET /auth/me` | проверка access-токена |

### Что важно (XSS-resistant pattern)

1. **Access token** — только в памяти (Zustand без `persist`). XSS видит его
   только пока вкладка открыта, и максимум 15 минут (TTL access токена).
2. **Refresh token** — **HttpOnly cookie**, JS прочитать не может. Браузер
   отправляет cookie сам на запросы к бэкенду (`credentials: 'include'`).
   Атрибуты: `HttpOnly`, `Secure`, `SameSite=Strict`, `Path=/`, `Max-Age=30d`.
   На XSS краже refresh токена не происходит.
3. **`SessionBootstrap`** при загрузке приложения тихо вызывает `/auth/refresh`
   — если cookie живая, восстанавливаем access токен и тянем `/auth/me`.
   Это обеспечивает "F5 не разлогинивает" при access-токене только в памяти.
4. **Single-flight в одной вкладке** + **Web Locks API между вкладками**:
   две вкладки не делают одновременно `/auth/refresh` — иначе reuse-detection
   бэкенда положит сессию (одна вкладка ротирует → вторая приходит со старым).
   `navigator.locks.request('auth-refresh', …)` сериализует.
5. **Auto-retry на 401**: один retry с новым токеном. Refresh упал →
   store очищается → `ProtectedRoute` редиректит на `/login`.
6. **CSRF**: не уязвимы — refresh cookie с `SameSite=Strict` отправляется
   только same-origin запросами. Bearer access токен вообще не из cookie.

## Структура

```
src/
├── main.tsx            # bootstrap + providers
├── App.tsx             # routes (lazy, code-split per route)
├── index.css           # Tailwind + CSS variables (light theme)
├── lib/
│   ├── api.ts          # fetch wrapper + single-flight + Web Lock refresh
│   ├── auth-store.ts   # Zustand in-memory (no persist)
│   └── utils.ts        # cn()
├── hooks/
│   └── use-auth.ts     # login / register / logout
├── components/
│   ├── session-bootstrap.tsx  # silent refresh on app mount
│   └── ui/             # Button, Input, Card
├── routes/
│   ├── protected-route.tsx
│   ├── login.tsx
│   ├── register.tsx
│   ├── dashboard.tsx
│   └── users.tsx
└── types/auth.ts       # User, AuthResponse, AccessResponse
```

## Бюджеты и CI гейты (от forcing-question grill)

Зафиксировано в Q1-Q7 grill сессии (см. `/tmp/frontend-grill-2026-06-10.md`):

| Метрика | Цель | Где проверять |
|---|---|---|
| LCP (p75) | ≤ 2500 ms | Lighthouse / RUM |
| INP (p75) | ≤ 200 ms | Lighthouse / RUM |
| CLS (p75) | ≤ 0.1 | Lighthouse / RUM |
| Initial JS | ≤ 200 KB gzip | `npm run size` |
| Per-route chunk | ≤ 80 KB gzip | `npm run size` |
| Lighthouse perf | ≥ 80 | CI (TODO) |
| Lighthouse a11y | ≥ 90 | CI (TODO) |
| WCAG | 2.2 AA | axe в e2e (TODO) |

### CI gates ещё не настроены

- `playwright + axe-core` для a11y smoke
- `lighthouse-ci` для perf budget на каждом PR
- `bundlewatch` или `size-limit` в GitHub Actions

Минимально для гейта по бандлу:
```bash
npm run build && npm run size
```

## Anti-patterns для этого профиля (vite-spa)

Запрещено (skill kill list):
- ❌ единый бандл без route-level code splitting — мёртвый старт на медленных каналах
- ❌ React Context как глобальный state — re-render каскады, используй Zustand
- ❌ SSR-обёртка над SPA-only поверхностью — инфра без пользы (нет SEO)
- ❌ Redux без необходимости — TanStack Query уже отвечает за server state

## Что не сделано (good follow-ups)

- Astro-приложение `/marketing` для SEO-индексируемого лендинга (Q5: hybrid)
- shadcn/ui полный init (Dialog, Toast, DropdownMenu) — пока минимум
- Playwright e2e + axe a11y check
- Lighthouse CI workflow
- Theme switch (light/dark) — CSS-vars готовы, нужна кнопка + media-query default
- HSTS header (требует реального HTTPS, не localhost)
