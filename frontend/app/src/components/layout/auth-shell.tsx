import { Outlet } from 'react-router-dom';

import { Header } from '@/components/layout/header';

/** Centered layout for login/register — shared header in guest state. */
export function AuthShell() {
  return (
    <div className="relative flex min-h-screen flex-col bg-background">
      {/* Subtle radial background accent */}
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-0 -z-10 opacity-50"
        style={{
          background:
            'radial-gradient(60% 60% at 50% 0%, hsl(var(--secondary)) 0%, transparent 70%)',
        }}
      />

      <Header />

      <main className="flex flex-1 items-center justify-center p-4">
        <Outlet />
      </main>

      <footer className="px-6 py-4 text-center text-xs text-muted-foreground">
        © {new Date().getFullYear()} Rust+React
      </footer>
    </div>
  );
}
