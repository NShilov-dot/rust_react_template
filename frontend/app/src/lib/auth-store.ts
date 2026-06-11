import { create } from 'zustand';

import type { User } from '@/types/auth';

interface AuthState {
  user: User | null;
  /**
   * Access token lives ONLY in memory — never localStorage.
   * On page reload it's gone; `SessionBootstrap` does a silent refresh via
   * the HttpOnly cookie to get a new one.
   */
  accessToken: string | null;
  accessExpiresAt: string | null;

  setSession: (user: User, accessToken: string, accessExpiresAt: string) => void;
  setAccess: (accessToken: string, accessExpiresAt: string) => void;
  setUser: (user: User) => void;
  clear: () => void;
}

/**
 * Auth store — in-memory only. The refresh token lives in an HttpOnly
 * cookie that JS cannot read, so XSS can't exfiltrate long-lived credentials.
 * The price: access token vanishes on F5 → `SessionBootstrap` re-acquires it.
 */
export const useAuthStore = create<AuthState>()((set) => ({
  user: null,
  accessToken: null,
  accessExpiresAt: null,

  setSession: (user, accessToken, accessExpiresAt) =>
    set({ user, accessToken, accessExpiresAt }),

  setAccess: (accessToken, accessExpiresAt) =>
    set({ accessToken, accessExpiresAt }),

  setUser: (user) => set({ user }),

  clear: () =>
    set({ user: null, accessToken: null, accessExpiresAt: null }),
}));

/** Read-only snapshot for use outside React (e.g. the fetch wrapper). */
export const authSnapshot = () => useAuthStore.getState();
