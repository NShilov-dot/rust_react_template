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
    <header className="sticky top-0 z-30 border-b border-border bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
      <div className="flex h-14 items-center justify-between gap-4 px-4">
        <div className="flex items-center gap-2">
          <SidebarToggle />
          <Brand />
        </div>

        <div className="flex items-center gap-3">
          <ThemeToggle />
          {isAuthed && user ? (
            <>
              <UserBadge user={user} />
              <Button
                variant="ghost"
                size="sm"
                onClick={handleLogout}
                aria-label="Выйти"
                title="Выйти"
              >
                <LogOut className="h-4 w-4" aria-hidden="true" />
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
    <NavLink to="/" className="flex items-center gap-2">
      <div
        aria-hidden="true"
        className="flex h-7 w-7 items-center justify-center rounded bg-primary text-xs font-bold text-primary-foreground"
      >
        R+
      </div>
      <span className="hidden text-sm font-semibold sm:inline">Rust+React</span>
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
            'inline-flex h-9 items-center gap-1.5 rounded-md px-3 text-sm font-medium transition-colors',
            'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring',
            isActive
              ? 'bg-secondary text-secondary-foreground'
              : 'text-muted-foreground hover:bg-accent hover:text-accent-foreground',
          )
        }
      >
        <LogIn className="h-4 w-4" aria-hidden="true" />
        <span>Войти</span>
      </NavLink>

      <NavLink
        to="/register"
        className={cn(
          'inline-flex h-9 items-center gap-1.5 rounded-md bg-primary px-3 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90',
          'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring',
        )}
      >
        <UserPlus className="h-4 w-4" aria-hidden="true" />
        <span className="hidden sm:inline">Регистрация</span>
      </NavLink>
    </nav>
  );
}
