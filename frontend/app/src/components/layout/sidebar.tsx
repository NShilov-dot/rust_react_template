import { useCallback, useEffect, useMemo, useState, type ReactNode } from 'react';
import { ChevronsLeft, LayoutDashboard, UserRound, Users, X } from 'lucide-react';
import { NavLink } from 'react-router-dom';

import { Button } from '@/components/ui/button';
import { useLocalStorage } from '@/hooks/use-local-storage';
import { SidebarContext, useSidebarStrict } from '@/hooks/use-sidebar';
import { cn } from '@/lib/utils';

const NAV = [
  { to: '/dashboard', label: 'Dashboard', icon: LayoutDashboard },
  { to: '/profile', label: 'Профиль', icon: UserRound },
  { to: '/users', label: 'Пользователи', icon: Users },
] as const;

export function SidebarProvider({ children }: { children: ReactNode }) {
  const [collapsed, setCollapsed] = useLocalStorage('sidebar:collapsed', false);
  const [mobileOpen, setMobileOpen] = useState(false);

  const toggleCollapsed = useCallback(() => setCollapsed((c) => !c), [setCollapsed]);

  const value = useMemo(
    () => ({ collapsed, toggleCollapsed, mobileOpen, setMobileOpen }),
    [collapsed, toggleCollapsed, mobileOpen],
  );

  return <SidebarContext.Provider value={value}>{children}</SidebarContext.Provider>;
}

export function Sidebar() {
  const { collapsed, toggleCollapsed, mobileOpen, setMobileOpen } = useSidebarStrict();

  // Close the mobile drawer on Escape.
  useEffect(() => {
    if (!mobileOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setMobileOpen(false);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [mobileOpen, setMobileOpen]);

  return (
    <>
      {/* Desktop: collapsible rail */}
      <aside
        aria-label="Боковая панель"
        className={cn(
          'sticky top-14 hidden h-[calc(100vh-3.5rem)] shrink-0 flex-col border-r border-border bg-background',
          'transition-[width] duration-300 ease-in-out motion-reduce:transition-none md:flex',
          collapsed ? 'w-14' : 'w-60',
        )}
      >
        <nav
          aria-label="Главная навигация"
          className="flex flex-1 flex-col gap-1 overflow-y-auto overflow-x-hidden p-2"
        >
          {NAV.map((item) => (
            <SidebarLink key={item.to} {...item} collapsed={collapsed} />
          ))}
        </nav>

        <div className="border-t border-border p-2">
          <button
            type="button"
            onClick={toggleCollapsed}
            aria-expanded={!collapsed}
            title={collapsed ? 'Развернуть' : 'Свернуть'}
            className={cn(
              'flex h-9 w-full items-center gap-2.5 overflow-hidden whitespace-nowrap rounded-md px-2.5 text-sm font-medium',
              'text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground',
              'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring',
            )}
          >
            <ChevronsLeft
              className={cn(
                'h-4 w-4 shrink-0 transition-transform duration-300 motion-reduce:transition-none',
                collapsed && 'rotate-180',
              )}
              aria-hidden="true"
            />
            <SidebarLabel collapsed={collapsed}>Свернуть</SidebarLabel>
          </button>
        </div>
      </aside>

      {/* Mobile: overlay + off-canvas drawer.
          `visible/invisible` keeps the exit animation and drops the hidden
          drawer from the a11y tree and tab order. */}
      <div
        aria-hidden="true"
        onClick={() => setMobileOpen(false)}
        className={cn(
          'fixed inset-0 z-40 bg-black/50 transition-[opacity,visibility] duration-300 motion-reduce:transition-none md:hidden',
          mobileOpen ? 'visible opacity-100' : 'invisible opacity-0',
        )}
      />
      <aside
        aria-label="Боковая панель"
        className={cn(
          'fixed inset-y-0 left-0 z-50 flex w-64 flex-col border-r border-border bg-background shadow-lg',
          'transition-[transform,visibility] duration-300 ease-in-out motion-reduce:transition-none md:hidden',
          mobileOpen ? 'visible translate-x-0' : 'invisible -translate-x-full',
        )}
      >
        <div className="flex h-14 shrink-0 items-center justify-between border-b border-border pl-4 pr-2">
          <span className="text-sm font-semibold">Меню</span>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setMobileOpen(false)}
            aria-label="Закрыть меню"
          >
            <X className="h-4 w-4" aria-hidden="true" />
          </Button>
        </div>
        <nav
          aria-label="Главная навигация"
          className="flex flex-1 flex-col gap-1 overflow-y-auto p-2"
        >
          {NAV.map((item) => (
            <SidebarLink
              key={item.to}
              {...item}
              collapsed={false}
              onNavigate={() => setMobileOpen(false)}
            />
          ))}
        </nav>
      </aside>
    </>
  );
}

function SidebarLink({
  to,
  label,
  icon: Icon,
  collapsed,
  onNavigate,
}: {
  to: string;
  label: string;
  icon: typeof LayoutDashboard;
  collapsed: boolean;
  onNavigate?: () => void;
}) {
  return (
    <NavLink
      to={to}
      onClick={onNavigate}
      title={collapsed ? label : undefined}
      className={({ isActive }) =>
        cn(
          'flex h-9 shrink-0 items-center gap-2.5 overflow-hidden whitespace-nowrap rounded-md px-2.5 text-sm font-medium transition-colors',
          'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring',
          isActive
            ? 'bg-secondary text-secondary-foreground'
            : 'text-muted-foreground hover:bg-accent hover:text-accent-foreground',
        )
      }
    >
      <Icon className="h-4 w-4 shrink-0" aria-hidden="true" />
      <SidebarLabel collapsed={collapsed}>{label}</SidebarLabel>
    </NavLink>
  );
}

/** Fades/slides the label while the rail width animates. */
function SidebarLabel({ collapsed, children }: { collapsed: boolean; children: ReactNode }) {
  return (
    <span
      className={cn(
        'truncate transition-[opacity,transform] duration-300 ease-in-out motion-reduce:transition-none',
        collapsed ? '-translate-x-2 opacity-0' : 'translate-x-0 opacity-100',
      )}
    >
      {children}
    </span>
  );
}
