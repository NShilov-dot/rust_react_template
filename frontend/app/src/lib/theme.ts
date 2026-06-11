import { create } from 'zustand';
import { persist } from 'zustand/middleware';

type ThemeMode = 'light' | 'dark' | 'system';

interface ThemeState {
  mode: ThemeMode;
  setMode: (mode: ThemeMode) => void;
}

export const useThemeStore = create<ThemeState>()(
  persist(
    (set) => ({
      mode: 'system',
      setMode: (mode) => set({ mode }),
    }),
    { name: 'theme' },
  ),
);

/** Apply the resolved theme to `<html>`. Called on every mode change. */
export function applyTheme(mode: ThemeMode) {
  const root = document.documentElement;
  const dark =
    mode === 'dark' ||
    (mode === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches);
  root.classList.toggle('dark', dark);
}

/** Wire the store to the DOM. Call once on app start. */
export function initTheme() {
  const apply = () => applyTheme(useThemeStore.getState().mode);
  apply();
  useThemeStore.subscribe(apply);
  // React to system preference changes when mode is "system".
  const mq = window.matchMedia('(prefers-color-scheme: dark)');
  mq.addEventListener('change', () => {
    if (useThemeStore.getState().mode === 'system') apply();
  });
}
