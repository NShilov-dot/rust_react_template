import { cn } from '@/lib/utils';
import type { User } from '@/types/auth';

interface Props {
  user: User;
  className?: string;
}

/**
 * Avatar (gradient + initials) + name + email — used in the authed header.
 * Wraps in a soft pill that picks up a subtle hover state, so the whole
 * area reads as a single interactive surface even though we don't open
 * a dropdown (the adjacent "Выйти" button handles the action).
 */
export function UserBadge({ user, className }: Props) {
  return (
    <div
      className={cn(
        'flex items-center gap-2.5 rounded-full border border-border/60 bg-card/40 py-1 pl-1 pr-1 transition-colors sm:pr-3.5',
        'hover:bg-accent/60',
        className,
      )}
    >
      <Avatar name={user.name} />
      <div className="hidden min-w-0 text-left sm:block">
        <div className="truncate text-sm font-medium leading-tight text-foreground">
          {user.name}
        </div>
        <div className="max-w-[16ch] truncate text-[11px] leading-tight text-muted-foreground">
          {user.email}
        </div>
      </div>
    </div>
  );
}

function Avatar({ name }: { name: string }) {
  const initials = initialsOf(name);
  return (
    <div
      aria-hidden="true"
      className={cn(
        'relative flex h-8 w-8 select-none items-center justify-center rounded-full text-xs font-semibold text-white',
        'bg-gradient-to-br from-amber-400 via-orange-500 to-rose-600',
        'shadow-inner ring-2 ring-background',
      )}
    >
      {initials}
    </div>
  );
}

function initialsOf(name: string): string {
  const parts = name.trim().split(/\s+/).slice(0, 2);
  return parts.map((p) => p[0]?.toUpperCase() ?? '').join('') || '?';
}
