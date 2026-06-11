import { LogIn, LogOut, Menu, PanelLeft, UserPlus } from 'lucide-react';
import { NavLink, useNavigate } from 'react-router-dom';

import { Button } from '@/components/ui/button';
import { ThemeToggle } from '@/components/theme-toggle';
import { UserBadge } from '@/components/layout/user-badge';
import { useAuth } from '@/hooks/use-auth';
import { useSidebar } from '@/hooks/use-sidebar';
import { cn } from '@/lib/utils';

/**
 * Shared header for both states:
 * - guest — brand + theme toggle + login/register links;
 * - authed — sidebar toggles + brand + theme toggle + user badge + logout.
 */
export function Header() {
  const { user, isAuthed, logout } = useAuth();
  const navigate = useNavigate();

  const handleLogout = async () => {
    await logout();
    navigate('/login', { replace: true });
  };

  return (
    <header className="sticky top-0 z-30 border-b border-border/60 bg-background/80 backdrop-blur-xl supports-[backdrop-filter]:bg-background/55">
      {/* Decorative top accent — hints at the Rust/React palette without painting the whole bar. */}
      <div
        aria-hidden="true"
        className="absolute inset-x-0 top-0 h-px bg-gradient-to-r from-transparent via-orange-500/50 to-transparent"
      />

      <div className="flex h-16 items-center justify-between gap-4 px-4 md:px-6">
        <div className="flex min-w-0 items-center gap-2">
          <SidebarToggle />
          <Brand />
        </div>

        <div className="flex items-center gap-2 md:gap-3">
          <ThemeToggle />
          {isAuthed && user ? (
            <>
              <span
                aria-hidden="true"
                className="hidden h-6 w-px bg-border/70 md:inline-block"
              />
              <UserBadge user={user} />
              <Button
                variant="ghost"
                size="sm"
                onClick={handleLogout}
                aria-label="Выйти"
                title="Выйти"
                className="group/logout text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
              >
                <LogOut
                  className="h-4 w-4 transition-transform duration-200 group-hover/logout:translate-x-0.5 motion-reduce:transition-none"
                  aria-hidden="true"
                />
                <span className="ml-1.5 hidden sm:inline">Выйти</span>
              </Button>
            </>
          ) : (
            <GuestNav />
          )}
        </div>
      </div>
    </header>
  );
}

/** Sidebar controls — rendered only inside SidebarProvider (app layout). */
function SidebarToggle() {
  const sidebar = useSidebar();
  if (!sidebar) return null;

  const { collapsed, toggleCollapsed, setMobileOpen } = sidebar;
  return (
    <>
      {/* Mobile: open the off-canvas drawer */}
      <Button
        variant="ghost"
        size="sm"
        className="md:hidden"
        onClick={() => setMobileOpen(true)}
        aria-label="Открыть меню"
      >
        <Menu className="h-4 w-4" aria-hidden="true" />
      </Button>

      {/* Desktop: collapse/expand the rail */}
      <Button
        variant="ghost"
        size="sm"
        className="hidden md:inline-flex"
        onClick={toggleCollapsed}
        aria-expanded={!collapsed}
        aria-label={collapsed ? 'Развернуть боковую панель' : 'Свернуть боковую панель'}
        title={collapsed ? 'Развернуть' : 'Свернуть'}
      >
        <PanelLeft
          className={cn(
            'h-4 w-4 transition-transform duration-300 motion-reduce:transition-none',
            collapsed && '-scale-x-100',
          )}
          aria-hidden="true"
        />
      </Button>
    </>
  );
}

function Brand() {
  return (
    <NavLink
      to="/"
      aria-label="На главную"
      className="group flex items-center gap-2.5 rounded-md outline-none ring-offset-2 ring-offset-background focus-visible:ring-2 focus-visible:ring-ring"
    >
      <div className="relative">
        <div
          aria-hidden="true"
          className={cn(
            'flex h-9 w-9 items-center justify-center rounded-lg text-[13px] font-extrabold text-white shadow-sm',
            'bg-gradient-to-br from-amber-400 via-orange-500 to-rose-600',
            'ring-1 ring-inset ring-white/15 transition-transform duration-300 motion-reduce:transition-none',
            'group-hover:scale-[1.04]',
          )}
        >
          R+
        </div>
        {/* Soft halo on hover. */}
        <span
          aria-hidden="true"
          className={cn(
            'pointer-events-none absolute inset-0 -z-10 rounded-lg blur-lg opacity-0 transition-opacity duration-300 motion-reduce:transition-none',
            'bg-gradient-to-br from-amber-400 via-orange-500 to-rose-600',
            'group-hover:opacity-50',
          )}
        />
      </div>

      <div className="hidden flex-col leading-none sm:flex">
        <span className="text-sm font-semibold tracking-tight text-foreground">
          Rust<span className="text-orange-500">+</span>React
        </span>
        <span className="mt-1 text-[10px] font-medium uppercase tracking-[0.18em] text-muted-foreground">
          auth scaffold
        </span>
      </div>
    </NavLink>
  );
}

function GuestNav() {
  return (
    <nav aria-label="Авторизация" className="flex items-center gap-1.5">
      <NavLink
        to="/login"
        className={({ isActive }) =>
          cn(
            'group/login inline-flex h-9 items-center gap-1.5 rounded-md px-3 text-sm font-medium transition-colors',
            'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring',
            isActive
              ? 'bg-secondary text-secondary-foreground'
              : 'text-muted-foreground hover:bg-accent hover:text-accent-foreground',
          )
        }
      >
        <LogIn
          className="h-4 w-4 transition-transform duration-200 group-hover/login:-translate-x-0.5 motion-reduce:transition-none"
          aria-hidden="true"
        />
        <span>Войти</span>
      </NavLink>

      <NavLink
        to="/register"
        className={cn(
          'group/register relative inline-flex h-9 items-center gap-1.5 overflow-hidden rounded-md px-3.5 text-sm font-medium text-white shadow-sm',
          'bg-gradient-to-br from-amber-500 via-orange-500 to-rose-600',
          'transition-all duration-200 hover:shadow-md hover:shadow-orange-500/20 motion-reduce:transition-none',
          'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-orange-500 focus-visible:ring-offset-2 focus-visible:ring-offset-background',
        )}
      >
        {/* Sheen sweep on hover */}
        <span
          aria-hidden="true"
          className="absolute inset-0 -translate-x-full bg-gradient-to-r from-transparent via-white/25 to-transparent transition-transform duration-500 group-hover/register:translate-x-full motion-reduce:transition-none"
        />
        <UserPlus className="relative h-4 w-4" aria-hidden="true" />
        <span className="relative hidden sm:inline">Регистрация</span>
      </NavLink>
    </nav>
  );
}
