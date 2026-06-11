import { useQuery } from '@tanstack/react-query';
import { Users as UsersIcon } from 'lucide-react';

import { Card } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import { api } from '@/lib/api';
import type { User } from '@/types/auth';

export default function UsersPage() {
  const usersQuery = useQuery({
    queryKey: ['users'],
    queryFn: () => api.get<User[]>('/users?limit=50'),
  });

  return (
    <div className="space-y-6">
      <header className="flex items-end justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Пользователи</h1>
          <p className="text-sm text-muted-foreground">
            Все зарегистрированные аккаунты
          </p>
        </div>
        {usersQuery.data && (
          <span className="text-xs text-muted-foreground">
            всего: {usersQuery.data.length}
          </span>
        )}
      </header>

      <Card className="overflow-hidden p-0">
        {usersQuery.isPending && <TableSkeleton />}

        {usersQuery.isError && (
          <p className="p-6 text-sm text-destructive" role="alert">
            Не удалось загрузить список пользователей
          </p>
        )}

        {usersQuery.data && usersQuery.data.length === 0 && <EmptyState />}

        {usersQuery.data && usersQuery.data.length > 0 && (
          <table className="w-full text-sm">
            <thead className="border-b border-border bg-muted/50 text-xs uppercase tracking-wide text-muted-foreground">
              <tr>
                <th scope="col" className="px-4 py-3 text-left font-medium">
                  Имя
                </th>
                <th scope="col" className="px-4 py-3 text-left font-medium">
                  Email
                </th>
                <th scope="col" className="px-4 py-3 text-left font-medium">
                  Создан
                </th>
              </tr>
            </thead>
            <tbody>
              {usersQuery.data.map((u) => (
                <tr
                  key={u.id}
                  className="border-b border-border last:border-0 hover:bg-muted/30"
                >
                  <td className="px-4 py-3 font-medium">{u.name}</td>
                  <td className="px-4 py-3 text-muted-foreground">{u.email}</td>
                  <td className="px-4 py-3 text-muted-foreground">
                    {new Date(u.created_at).toLocaleDateString()}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </Card>
    </div>
  );
}

function TableSkeleton() {
  return (
    <div className="divide-y divide-border">
      {Array.from({ length: 5 }).map((_, i) => (
        <div key={i} className="flex items-center gap-4 px-4 py-3">
          <Skeleton className="h-4 w-32" />
          <Skeleton className="h-4 flex-1" />
          <Skeleton className="h-4 w-20" />
        </div>
      ))}
    </div>
  );
}

function EmptyState() {
  return (
    <div className="flex flex-col items-center gap-3 px-6 py-12 text-center">
      <div className="flex h-12 w-12 items-center justify-center rounded-full bg-secondary">
        <UsersIcon className="h-5 w-5 text-secondary-foreground" aria-hidden="true" />
      </div>
      <p className="text-sm font-medium">Пока никого нет</p>
      <p className="text-sm text-muted-foreground">
        Зарегистрируйте первого пользователя, чтобы он появился здесь.
      </p>
    </div>
  );
}
