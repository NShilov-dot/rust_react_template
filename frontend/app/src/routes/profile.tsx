import { useQuery } from '@tanstack/react-query';
import { CalendarDays, Fingerprint, Mail, RefreshCw, ShieldCheck, UserRound } from 'lucide-react';

import { Card } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import { api } from '@/lib/api';
import type { User } from '@/types/auth';

export default function ProfilePage() {
  const meQuery = useQuery({
    queryKey: ['me'],
    queryFn: () => api.get<User>('/auth/me'),
  });

  return (
    <div className="space-y-6">
      <header>
        <h1 className="text-2xl font-semibold tracking-tight">Профиль</h1>
        <p className="text-sm text-muted-foreground">Данные вашего аккаунта</p>
      </header>

      {meQuery.isPending && <ProfileSkeleton />}

      {meQuery.isError && (
        <Card className="p-6">
          <p className="text-sm text-destructive" role="alert">
            Не удалось получить профиль.
          </p>
        </Card>
      )}

      {meQuery.data && (
        <>
          <Card className="p-6">
            <div className="flex items-center gap-4">
              <Avatar name={meQuery.data.name} />
              <div>
                <div className="text-lg font-semibold leading-tight">
                  {meQuery.data.name}
                </div>
                <div className="text-sm text-muted-foreground">{meQuery.data.email}</div>
              </div>
            </div>
          </Card>

          <section
            aria-label="Данные аккаунта"
            className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3"
          >
            <Field icon={UserRound} label="Имя" value={meQuery.data.name} />
            <Field icon={Mail} label="Email" value={meQuery.data.email} />
            <Field icon={Fingerprint} label="ID" value={meQuery.data.id} />
            <Field
              icon={CalendarDays}
              label="Аккаунт создан"
              value={formatDateTime(meQuery.data.created_at)}
            />
            <Field
              icon={RefreshCw}
              label="Обновлён"
              value={formatDateTime(meQuery.data.updated_at)}
            />
          </section>
        </>
      )}

      <Card className="p-6">
        <div className="flex items-start gap-4">
          <div
            aria-hidden="true"
            className="flex h-10 w-10 shrink-0 items-center justify-center rounded-md bg-secondary"
          >
            <ShieldCheck className="h-5 w-5 text-secondary-foreground" />
          </div>
          <div className="space-y-1">
            <h2 className="text-base font-medium">Сессия защищена</h2>
            <p className="text-sm text-muted-foreground">
              Refresh-токен в HttpOnly cookie, access-токен только в памяти,
              rotation с reuse-detection на бэкенде.
            </p>
          </div>
        </div>
      </Card>
    </div>
  );
}

function Avatar({ name }: { name: string }) {
  const initials =
    name
      .trim()
      .split(/\s+/)
      .slice(0, 2)
      .map((p) => p[0]?.toUpperCase() ?? '')
      .join('') || '?';

  return (
    <div
      aria-hidden="true"
      className="flex h-14 w-14 select-none items-center justify-center rounded-full bg-primary text-lg font-semibold text-primary-foreground"
    >
      {initials}
    </div>
  );
}

function Field({
  icon: Icon,
  label,
  value,
}: {
  icon: typeof UserRound;
  label: string;
  value: string;
}) {
  return (
    <Card className="p-5">
      <div className="flex items-center gap-2 text-xs uppercase tracking-wide text-muted-foreground">
        <Icon className="h-3.5 w-3.5" aria-hidden="true" />
        {label}
      </div>
      <div className="mt-2 truncate text-base font-medium" title={value}>
        {value}
      </div>
    </Card>
  );
}

function formatDateTime(iso: string): string {
  return new Date(iso).toLocaleString();
}

function ProfileSkeleton() {
  return (
    <div className="space-y-6">
      <Card className="p-6">
        <div className="flex items-center gap-4">
          <Skeleton className="h-14 w-14 rounded-full" />
          <div className="space-y-2">
            <Skeleton className="h-5 w-40" />
            <Skeleton className="h-4 w-56" />
          </div>
        </div>
      </Card>
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {Array.from({ length: 5 }).map((_, i) => (
          <Card key={i} className="p-5">
            <Skeleton className="h-3 w-20" />
            <Skeleton className="mt-3 h-5 w-32" />
          </Card>
        ))}
      </div>
    </div>
  );
}
