import { cn } from '@/lib/utils';
import type { User } from '@/types/auth';

interface Props {
  user: User;
  className?: string;
}

/** Avatar (initials) + name + email — used in the header. */
export function UserBadge({ user, className }: Props) {
  return (
    <div className={cn('flex items-center gap-2', className)}>
      <Avatar name={user.name} />
      <div className="hidden text-left sm:block">
        <div className="text-sm font-medium leading-tight">{user.name}</div>
        <div className="text-xs leading-tight text-muted-foreground">{user.email}</div>
      </div>
    </div>
  );
}

function Avatar({ name }: { name: string }) {
  const initials = initialsOf(name);
  return (
    <div
      aria-hidden="true"
      className="flex h-8 w-8 select-none items-center justify-center rounded-full bg-primary text-xs font-semibold text-primary-foreground"
    >
      {initials}
    </div>
  );
}

function initialsOf(name: string): string {
  const parts = name.trim().split(/\s+/).slice(0, 2);
  return parts.map((p) => p[0]?.toUpperCase() ?? '').join('') || '?';
}
