import { authSnapshot, useAuthStore } from '@/lib/auth-store';
import type { AccessResponse, ApiErrorBody } from '@/types/auth';

const API_BASE = '/api';

export class ApiError extends Error {
  constructor(
    public status: number,
    public body: ApiErrorBody | null,
  ) {
    super(body?.error ?? `HTTP ${status}`);
    this.name = 'ApiError';
  }
}

interface RequestOpts {
  method?: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';
  body?: unknown;
  /** Skip Authorization header (for /auth/login etc.). */
  anonymous?: boolean;
  signal?: AbortSignal;
}

// ─── Multi-tab refresh coordination ───────────────────────────────────
// Within a tab: a Promise ref gates concurrent refresh calls.
// Across tabs: Web Locks API serializes refreshes, so two tabs don't race
// against the backend's reuse-detection and kill the session.
let refreshPromise: Promise<AccessResponse> | null = null;

async function withCrossTabLock<T>(name: string, fn: () => Promise<T>): Promise<T> {
  if (typeof navigator !== 'undefined' && 'locks' in navigator) {
    return await navigator.locks.request(name, { mode: 'exclusive' }, async () => fn());
  }
  return await fn();
}

async function refreshTokens(): Promise<AccessResponse> {
  if (refreshPromise) return refreshPromise;

  refreshPromise = withCrossTabLock('auth-refresh', async () => {
    // Cookie carries the refresh token; no body, no Authorization header.
    const res = await fetch(`${API_BASE}/auth/refresh`, {
      method: 'POST',
      credentials: 'include',
    });
    if (!res.ok) {
      useAuthStore.getState().clear();
      throw new ApiError(res.status, await parseError(res));
    }
    const data = (await res.json()) as AccessResponse;
    useAuthStore.getState().setAccess(data.access_token, data.access_expires_at);
    return data;
  }).finally(() => {
    refreshPromise = null;
  });

  return refreshPromise;
}

async function parseError(res: Response): Promise<ApiErrorBody | null> {
  try {
    return (await res.json()) as ApiErrorBody;
  } catch {
    return null;
  }
}

async function rawRequest(
  path: string,
  opts: RequestOpts,
  token: string | null,
): Promise<Response> {
  const headers: Record<string, string> = {};
  if (opts.body !== undefined) headers['content-type'] = 'application/json';
  if (token && !opts.anonymous) headers['authorization'] = `Bearer ${token}`;

  return fetch(`${API_BASE}${path}`, {
    method: opts.method ?? 'GET',
    headers,
    body: opts.body !== undefined ? JSON.stringify(opts.body) : undefined,
    signal: opts.signal,
    // `include` ensures the HttpOnly refresh cookie ships with every request
    // and that Set-Cookie from the server is honored. Same-origin works with
    // 'same-origin' too, but being explicit avoids surprises if origins ever
    // diverge.
    credentials: 'include',
  });
}

export async function request<T = unknown>(
  path: string,
  opts: RequestOpts = {},
): Promise<T> {
  const initialToken = opts.anonymous ? null : authSnapshot().accessToken;
  let res = await rawRequest(path, opts, initialToken);

  // On 401 for an authed request, try one silent refresh + replay.
  if (res.status === 401 && !opts.anonymous) {
    try {
      const fresh = await refreshTokens();
      res = await rawRequest(path, opts, fresh.access_token);
    } catch {
      // refresh already cleared the store; bubble the original 401
    }
  }

  if (!res.ok) throw new ApiError(res.status, await parseError(res));

  if (res.status === 204) return undefined as T;
  return (await res.json()) as T;
}

export const api = {
  get: <T = unknown>(path: string, opts?: Omit<RequestOpts, 'method' | 'body'>) =>
    request<T>(path, { ...opts, method: 'GET' }),

  post: <T = unknown>(
    path: string,
    body?: unknown,
    opts?: Omit<RequestOpts, 'method' | 'body'>,
  ) => request<T>(path, { ...opts, method: 'POST', body }),

  delete: <T = unknown>(path: string, opts?: Omit<RequestOpts, 'method' | 'body'>) =>
    request<T>(path, { ...opts, method: 'DELETE' }),
};

/** Exposed so `SessionBootstrap` can attempt the initial silent refresh. */
export const tryRefresh = () => refreshTokens();
