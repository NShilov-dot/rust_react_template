import { useQuery } from '@tanstack/react-query';
import { ArrowRight, UserRound, Users } from 'lucide-react';
import { Link } from 'react-router-dom';

import { Card } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import { api } from '@/lib/api';
import { useAuth } from '@/hooks/use-auth';
import type { User } from '@/types/auth';

export default function DashboardPage() {
  const { user } = useAuth();
  const usersQuery = useQuery({
    queryKey: ['users'],
    queryFn: () => api.get<User[]>('/users?limit=50'),
  });

  return (
    <div className="space-y-6">
      <header>
        <h1 className="text-2xl font-semibold tracking-tight">Добро пожаловать</h1>
        <p className="text-sm text-muted-foreground">
          {user ? `${user.name}, рады видеть вас снова.` : 'Рады видеть вас снова.'}
        </p>
      </header>

      <section
        aria-label="Сводка"
        className="grid grid-cols-1 gap-4 sm:grid-cols-2"
      >
        <Card className="p-5">
          <div className="flex items-center gap-2 text-xs uppercase tracking-wide text-muted-foreground">
            <Users className="h-3.5 w-3.5" aria-hidden="true" />
            Пользователей в системе
          </div>
          {usersQuery.isPending ? (
            <Skeleton className="mt-3 h-7 w-12" />
          ) : (
            <div className="mt-2 text-2xl font-semibold">
              {usersQuery.data?.length ?? '—'}
            </div>
          )}
        </Card>

        <Card className="p-5">
          <div className="flex items-center gap-2 text-xs uppercase tracking-wide text-muted-foreground">
            <UserRound className="h-3.5 w-3.5" aria-hidden="true" />
            Ваш аккаунт
          </div>
          <div className="mt-2 truncate text-2xl font-semibold" title={user?.email}>
            {user?.email ?? '—'}
          </div>
        </Card>
      </section>

      <section
        aria-label="Быстрые переходы"
        className="grid grid-cols-1 gap-4 sm:grid-cols-2"
      >
        <QuickLink
          to="/profile"
          icon={UserRound}
          title="Профиль"
          description="Данные вашего аккаунта: имя, email, даты регистрации и обновления."
        />
        <QuickLink
          to="/users"
          icon={Users}
          title="Пользователи"
          description="Список всех зарегистрированных аккаунтов."
        />
      </section>
    </div>
  );
}

function QuickLink({
  to,
  icon: Icon,
  title,
  description,
}: {
  to: string;
  icon: typeof UserRound;
  title: string;
  description: string;
}) {
  return (
    <Link to={to} className="group focus-visible:outline-none">
      <Card className="h-full p-6 transition-colors group-hover:bg-accent/50 group-focus-visible:ring-2 group-focus-visible:ring-ring">
        <div className="flex items-start gap-4">
          <div
            aria-hidden="true"
            className="flex h-10 w-10 shrink-0 items-center justify-center rounded-md bg-secondary"
          >
            <Icon className="h-5 w-5 text-secondary-foreground" />
          </div>
          <div className="space-y-1">
            <h2 className="flex items-center gap-1.5 text-base font-medium">
              {title}
              <ArrowRight
                className="h-4 w-4 transition-transform group-hover:translate-x-0.5"
                aria-hidden="true"
              />
            </h2>
            <p className="text-sm text-muted-foreground">{description}</p>
          </div>
        </div>
      </Card>
    </Link>
  );
}
