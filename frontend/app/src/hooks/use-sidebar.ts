import { createContext, useContext } from 'react';

export interface SidebarState {
  /** Desktop rail collapsed to icons-only. Persisted in localStorage. */
  collapsed: boolean;
  toggleCollapsed: () => void;
  /** Mobile off-canvas drawer visibility. */
  mobileOpen: boolean;
  setMobileOpen: (open: boolean) => void;
}

export const SidebarContext = createContext<SidebarState | null>(null);

/** Null on layouts without a sidebar (e.g. auth pages). */
export function useSidebar(): SidebarState | null {
  return useContext(SidebarContext);
}

export function useSidebarStrict(): SidebarState {
  const ctx = useContext(SidebarContext);
  if (!ctx) throw new Error('<Sidebar> must be rendered inside <SidebarProvider>');
  return ctx;
}
