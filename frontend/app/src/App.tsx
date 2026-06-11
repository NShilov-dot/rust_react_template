import { lazy, Suspense, useEffect } from 'react';
import { Navigate, Route, Routes } from 'react-router-dom';

import { AppShell } from '@/components/layout/app-shell';
import { AuthShell } from '@/components/layout/auth-shell';
import { GuestRoute } from '@/routes/guest-route';
import { ProtectedRoute } from '@/routes/protected-route';
import { initTheme } from '@/lib/theme';

const LoginPage = lazy(() => import('@/routes/login'));
const RegisterPage = lazy(() => import('@/routes/register'));
const DashboardPage = lazy(() => import('@/routes/dashboard'));
const ProfilePage = lazy(() => import('@/routes/profile'));
const UsersPage = lazy(() => import('@/routes/users'));

export function App() {
  useEffect(() => {
    initTheme();
  }, []);

  return (
    <Suspense fallback={<RouteFallback />}>
      <Routes>
        <Route element={<GuestRoute />}>
          <Route element={<AuthShell />}>
            <Route path="/login" element={<LoginPage />} />
            <Route path="/register" element={<RegisterPage />} />
          </Route>
        </Route>

        <Route element={<ProtectedRoute />}>
          <Route element={<AppShell />}>
            <Route path="/dashboard" element={<DashboardPage />} />
            <Route path="/profile" element={<ProfilePage />} />
            <Route path="/users" element={<UsersPage />} />
          </Route>
        </Route>

        <Route path="/" element={<Navigate to="/dashboard" replace />} />
        <Route path="*" element={<Navigate to="/dashboard" replace />} />
      </Routes>
    </Suspense>
  );
}

function RouteFallback() {
  return (
    <div
      className="flex min-h-screen items-center justify-center text-muted-foreground"
      role="status"
      aria-live="polite"
    >
      Loading…
    </div>
  );
}
