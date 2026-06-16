import { useCallback } from 'react';

import { useQueryClient } from '@tanstack/react-query';

import { api } from '@/lib/api';
import { useAuthStore } from '@/lib/auth-store';
import type { AuthResponse } from '@/types/auth';

export function useAuth() {
  const { user, accessToken, setSession, clear } = useAuthStore();
  const queryClient = useQueryClient();
  const isAuthed = Boolean(accessToken && user);

  const login = useCallback(
    async (email: string, password: string) => {
      const res = await api.post<AuthResponse>(
        '/auth/login',
        { email, password },
        { anonymous: true },
      );
      setSession(res.user, res.access_token, res.access_expires_at);
      return res;
    },
    [setSession],
  );

  const register = useCallback(
    async (email: string, name: string, password: string) => {
      const res = await api.post<AuthResponse>(
        '/auth/register',
        { email, name, password },
        { anonymous: true },
      );
      setSession(res.user, res.access_token, res.access_expires_at);
      return res;
    },
    [setSession],
  );

  const logout = useCallback(async () => {
    try {
      // Cookie carries the refresh token; backend revokes + clears it.
      await api.post('/auth/logout');
    } catch {
      // best-effort; we clear locally either way
    } finally {
      // Clear auth state first, then purge the query cache. Order matters:
      // ProtectedRoute reads from the store — clearing it first ensures any
      // in-flight re-renders see "logged out" before stale query data is gone.
      clear();
      // Wipe all cached server state so a subsequent login as a different user
      // never sees a prior user's /auth/me, /users, /tasks responses.
      queryClient.clear();
    }
  }, [clear, queryClient]);

  return { user, isAuthed, login, register, logout };
}
