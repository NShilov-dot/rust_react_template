import { Navigate, Outlet, useLocation } from 'react-router-dom';

import { useAuthStore } from '@/lib/auth-store';

interface LocationState {
  from?: string;
}

/**
 * Mirror of `ProtectedRoute` for /login and /register: if a session already
 * exists, send the user to wherever they were heading (preserved via
 * `location.state.from` when ProtectedRoute bounced them), or fall back
 * to /dashboard.
 *
 * Safe because `SessionBootstrap` waits for the silent refresh to settle
 * before rendering the router, so `accessToken` reflects ground truth here.
 */
export function GuestRoute() {
  const accessToken = useAuthStore((s) => s.accessToken);
  const location = useLocation();

  if (accessToken) {
    const from = (location.state as LocationState | null)?.from ?? '/dashboard';
    return <Navigate to={from} replace />;
  }

  return <Outlet />;
}
