import { Monitor, Moon, Sun } from 'lucide-react';

import { useThemeStore } from '@/lib/theme';
import { cn } from '@/lib/utils';

const OPTIONS = [
  { value: 'light', icon: Sun, label: 'Светлая' },
  { value: 'system', icon: Monitor, label: 'Системная' },
  { value: 'dark', icon: Moon, label: 'Тёмная' },
] as const;

export function ThemeToggle() {
  const mode = useThemeStore((s) => s.mode);
  const setMode = useThemeStore((s) => s.setMode);

  return (
    <div
      role="radiogroup"
      aria-label="Тема"
      className="inline-flex items-center gap-0.5 rounded-md border border-border bg-background p-0.5"
    >
      {OPTIONS.map(({ value, icon: Icon, label }) => (
        <button
          key={value}
          type="button"
          role="radio"
          aria-checked={mode === value}
          aria-label={label}
          title={label}
          onClick={() => setMode(value)}
          className={cn(
            'inline-flex h-7 w-7 items-center justify-center rounded transition-colors',
            'hover:bg-accent hover:text-accent-foreground',
            mode === value && 'bg-secondary text-secondary-foreground',
          )}
        >
          <Icon className="h-3.5 w-3.5" aria-hidden="true" />
        </button>
      ))}
    </div>
  );
}
