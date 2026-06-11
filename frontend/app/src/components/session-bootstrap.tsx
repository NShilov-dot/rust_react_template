import { useEffect, useState } from 'react';

import { api, tryRefresh } from '@/lib/api';
import { useAuthStore } from '@/lib/auth-store';
import type { User } from '@/types/auth';

/**
 * On app mount, try a silent refresh against the HttpOnly cookie. If a session
 * exists, fetch `/auth/me` and seed the store. If not, render anonymously and
 * let `ProtectedRoute` redirect to /login when needed.
 *
 * This is what makes "F5 keeps me logged in" work with in-memory access tokens.
 */
export function SessionBootstrap({ children }: { children: React.ReactNode }) {
  const accessToken = useAuthStore((s) => s.accessToken);
  const [ready, setReady] = useState(Boolean(accessToken));

  useEffect(() => {
    if (accessToken) {
      setReady(true);
      return;
    }

    let cancelled = false;
    (async () => {
      try {
        await tryRefresh();
        const me = await api.get<User>('/auth/me');
        if (!cancelled) useAuthStore.getState().setUser(me);
      } catch {
        // No session — fine. Anonymous routes still render.
      } finally {
        if (!cancelled) setReady(true);
      }
    })();

    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  if (!ready) {
    return (
      <div
        className="flex min-h-screen items-center justify-center text-muted-foreground"
        role="status"
        aria-live="polite"
      >
        Восстанавливаем сессию…
      </div>
    );
  }

  return <>{children}</>;
}
