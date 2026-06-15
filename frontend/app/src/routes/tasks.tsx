import { useMemo, useState } from 'react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useAutoAnimate } from '@formkit/auto-animate/react';
import {
  CalendarDays,
  CheckCircle2,
  CircleDashed,
  ClipboardList,
  Loader2,
  Pencil,
  Plus,
  Trash2,
  X,
} from 'lucide-react';
import { Virtuoso } from 'react-virtuoso';
import { z } from 'zod';

import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Skeleton } from '@/components/ui/skeleton';
import { ApiError } from '@/lib/api';
import {
  useCreateTask,
  useDeleteTask,
  useTasks,
  useUpdateTask,
} from '@/lib/tasks-api';
import { cn } from '@/lib/utils';
import type { Task, TaskPriority, TaskStatus } from '@/types/tasks';

/** Shared auto-animate timing. `respectMotionPreference` is default-on. */
const ANIMATE_OPTS = { duration: 180, easing: 'ease-out' } as const;

// ─── Labels & visual mappings ──────────────────────────────────────────

const STATUS_LABEL: Record<TaskStatus, string> = {
  todo: 'К выполнению',
  in_progress: 'В работе',
  done: 'Готово',
};

const PRIORITY_LABEL: Record<TaskPriority, string> = {
  low: 'Низкий',
  medium: 'Средний',
  high: 'Высокий',
};

const PRIORITY_TONE: Record<TaskPriority, string> = {
  low: 'bg-muted text-muted-foreground',
  medium: 'bg-secondary text-secondary-foreground',
  high: 'bg-destructive/15 text-destructive',
};

const STATUS_FILTERS: Array<{ value: TaskStatus | 'all'; label: string }> = [
  { value: 'all', label: 'Все' },
  { value: 'todo', label: STATUS_LABEL.todo },
  { value: 'in_progress', label: STATUS_LABEL.in_progress },
  { value: 'done', label: STATUS_LABEL.done },
];

// ─── Form schema ───────────────────────────────────────────────────────

const createSchema = z.object({
  title: z.string().trim().min(1, 'Введите название').max(200, 'Слишком длинное название'),
  description: z.string().trim().max(5000).optional().or(z.literal('')),
  priority: z.enum(['low', 'medium', 'high']),
  due_date: z.string().optional().or(z.literal('')),
});

type CreateValues = z.infer<typeof createSchema>;

const editSchema = z.object({
  title: z.string().trim().min(1, 'Введите название').max(200),
  description: z.string().trim().max(5000).optional().or(z.literal('')),
  priority: z.enum(['low', 'medium', 'high']),
  due_date: z.string().optional().or(z.literal('')),
});

type EditValues = z.infer<typeof editSchema>;

// ─── Date helpers ──────────────────────────────────────────────────────

function dueDateToIso(yyyyMmDd: string | undefined | null): string | undefined {
  if (!yyyyMmDd) return undefined;
  const d = new Date(`${yyyyMmDd}T23:59:59`);
  if (Number.isNaN(d.getTime())) return undefined;
  return d.toISOString();
}

function isoToDateInput(iso: string | null): string {
  if (!iso) return '';
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return '';
  const yyyy = d.getFullYear();
  const mm = String(d.getMonth() + 1).padStart(2, '0');
  const dd = String(d.getDate()).padStart(2, '0');
  return `${yyyy}-${mm}-${dd}`;
}

function formatDueDate(iso: string): string {
  return new Date(iso).toLocaleDateString();
}

// ─── Page ──────────────────────────────────────────────────────────────

export default function TasksPage() {
  const [filter, setFilter] = useState<TaskStatus | 'all'>('all');
  const [showCreate, setShowCreate] = useState(false);

  const tasksQuery = useTasks(filter === 'all' ? {} : { status: filter });

  // Flatten loaded pages into one array. The `useMemo` keeps a stable
  // reference between renders so Virtuoso doesn't fully re-render the list
  // on every parent re-render.
  const tasks = useMemo(
    () => tasksQuery.data?.pages.flatMap((p) => p.tasks) ?? [],
    [tasksQuery.data],
  );

  // Only the composer (create-form expand/collapse) uses auto-animate.
  // The task list is virtualized by Virtuoso, which recycles DOM nodes
  // as the user scrolls — putting auto-animate on a virtualized list
  // would trigger animations on row recycling.
  const [composerRef] = useAutoAnimate<HTMLDivElement>(ANIMATE_OPTS);

  return (
    <div className="space-y-6">
      <header className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Мои задачи</h1>
          <p className="text-sm text-muted-foreground">
            Личный backlog. Создавайте, ведите статусы, расставляйте приоритеты.
          </p>
        </div>
        <Button
          onClick={() => setShowCreate((v) => !v)}
          variant={showCreate ? 'outline' : 'default'}
          aria-expanded={showCreate}
        >
          {showCreate ? (
            <>
              <X className="mr-2 h-4 w-4" aria-hidden="true" />
              Отмена
            </>
          ) : (
            <>
              <Plus className="mr-2 h-4 w-4" aria-hidden="true" />
              Новая задача
            </>
          )}
        </Button>
      </header>

      <div ref={composerRef}>
        {showCreate && <CreateTaskForm onDone={() => setShowCreate(false)} />}
      </div>

      <FilterTabs current={filter} onChange={setFilter} />

      <TaskList
        tasks={tasks}
        isPending={tasksQuery.isPending}
        isError={tasksQuery.isError}
        hasNextPage={tasksQuery.hasNextPage}
        isFetchingNextPage={tasksQuery.isFetchingNextPage}
        onEndReached={() => {
          if (tasksQuery.hasNextPage && !tasksQuery.isFetchingNextPage) {
            void tasksQuery.fetchNextPage();
          }
        }}
      />
    </div>
  );
}

// ─── Filter tabs ───────────────────────────────────────────────────────

function FilterTabs({
  current,
  onChange,
}: {
  current: TaskStatus | 'all';
  onChange: (s: TaskStatus | 'all') => void;
}) {
  return (
    <div
      role="tablist"
      aria-label="Фильтр по статусу"
      className="inline-flex flex-wrap items-center gap-1 rounded-md border border-border bg-background p-1"
    >
      {STATUS_FILTERS.map((f) => {
        const active = current === f.value;
        return (
          <button
            key={f.value}
            role="tab"
            aria-selected={active}
            onClick={() => onChange(f.value)}
            className={cn(
              'inline-flex h-8 items-center rounded-md px-3 text-sm font-medium transition-colors',
              'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring',
              active
                ? 'bg-secondary text-secondary-foreground'
                : 'text-muted-foreground hover:bg-accent hover:text-accent-foreground',
            )}
          >
            {f.label}
          </button>
        );
      })}
    </div>
  );
}

// ─── Virtualized list ──────────────────────────────────────────────────

function TaskList({
  tasks,
  isPending,
  isError,
  hasNextPage,
  isFetchingNextPage,
  onEndReached,
}: {
  tasks: Task[];
  isPending: boolean;
  isError: boolean;
  hasNextPage: boolean;
  isFetchingNextPage: boolean;
  onEndReached: () => void;
}) {
  if (isPending) return <TaskListSkeleton />;

  if (isError) {
    return (
      <Card className="p-6">
        <p className="text-sm text-destructive" role="alert">
          Не удалось загрузить задачи
        </p>
      </Card>
    );
  }

  if (tasks.length === 0) return <EmptyState />;

  return (
    <section aria-label="Список задач">
      <Virtuoso
        // `useWindowScroll` makes Virtuoso piggyback on the page's normal
        // scroll instead of needing a fixed-height container — keeps the
        // layout simple and the header/filter accessible while scrolling.
        useWindowScroll
        data={tasks}
        // Stable keys so React (and the virtualizer) reuse rows when the
        // underlying array changes order. Optimistic-temp rows are swapped
        // for their real twins on success, which is one row change, not a
        // whole-list re-mount.
        computeItemKey={(_, task) => task.id}
        itemContent={(_, task) => (
          <div className="pb-3">
            <TaskCard task={task} />
          </div>
        )}
        // 600 px overscan keeps two screens of cards rendered above/below
        // the viewport — smooth scrolling, modest memory.
        overscan={600}
        endReached={onEndReached}
        increaseViewportBy={300}
        components={{
          Footer: () =>
            hasNextPage ? (
              <div
                className="flex items-center justify-center py-4 text-sm text-muted-foreground"
                role="status"
              >
                <Loader2
                  className={cn(
                    'mr-2 h-4 w-4',
                    isFetchingNextPage && 'animate-spin',
                  )}
                  aria-hidden="true"
                />
                {isFetchingNextPage ? 'Загружаем…' : 'Прокрутите вниз'}
              </div>
            ) : (
              <div className="py-4 text-center text-xs text-muted-foreground">
                Это все задачи
              </div>
            ),
        }}
      />
    </section>
  );
}

// ─── Create form ───────────────────────────────────────────────────────

function CreateTaskForm({ onDone }: { onDone: () => void }) {
  const create = useCreateTask({ onSuccess: () => onDone() });

  const {
    register,
    handleSubmit,
    formState: { errors, isSubmitting },
    reset,
  } = useForm<CreateValues>({
    resolver: zodResolver(createSchema),
    defaultValues: { title: '', description: '', priority: 'medium', due_date: '' },
  });

  const onSubmit = handleSubmit(async (values) => {
    await create.mutateAsync({
      title: values.title,
      description: values.description?.trim() ? values.description : undefined,
      priority: values.priority,
      due_date: dueDateToIso(values.due_date),
    });
    reset();
  });

  const serverError = create.error instanceof ApiError ? create.error.message : null;

  return (
    <Card className="p-5">
      <form onSubmit={onSubmit} className="space-y-3" noValidate>
        <div>
          <label className="mb-1 block text-sm font-medium" htmlFor="task-title">
            Название
          </label>
          <Input
            id="task-title"
            placeholder="Например: Подготовить демо MVP"
            aria-invalid={!!errors.title}
            error={errors.title?.message}
            {...register('title')}
          />
        </div>

        <div>
          <label className="mb-1 block text-sm font-medium" htmlFor="task-description">
            Описание (опционально)
          </label>
          <textarea
            id="task-description"
            rows={3}
            placeholder="Что нужно сделать в деталях"
            className={cn(
              'flex w-full rounded-md border border-input bg-background px-3 py-2 text-sm',
              'placeholder:text-muted-foreground',
              'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring',
              'disabled:cursor-not-allowed disabled:opacity-50',
            )}
            {...register('description')}
          />
        </div>

        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
          <div>
            <label className="mb-1 block text-sm font-medium" htmlFor="task-priority">
              Приоритет
            </label>
            <select
              id="task-priority"
              className={cn(
                'flex h-10 w-full rounded-md border border-input bg-background px-3 text-sm',
                'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring',
              )}
              {...register('priority')}
            >
              <option value="low">{PRIORITY_LABEL.low}</option>
              <option value="medium">{PRIORITY_LABEL.medium}</option>
              <option value="high">{PRIORITY_LABEL.high}</option>
            </select>
          </div>
          <div>
            <label className="mb-1 block text-sm font-medium" htmlFor="task-due">
              Срок (опционально)
            </label>
            <Input id="task-due" type="date" {...register('due_date')} />
          </div>
        </div>

        {serverError && (
          <p className="text-sm text-destructive" role="alert">
            {serverError}
          </p>
        )}

        <div className="flex justify-end gap-2 pt-1">
          <Button type="button" variant="ghost" onClick={onDone}>
            Отмена
          </Button>
          <Button type="submit" disabled={isSubmitting}>
            {isSubmitting ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" aria-hidden="true" />
                Создаём…
              </>
            ) : (
              'Создать'
            )}
          </Button>
        </div>
      </form>
    </Card>
  );
}

// ─── Task card ─────────────────────────────────────────────────────────

function TaskCard({ task }: { task: Task }) {
  const [editing, setEditing] = useState(false);
  const update = useUpdateTask();
  const del = useDeleteTask();

  // Temp rows (from optimistic create) have a synthetic id and can't be
  // mutated on the server yet. Lock controls until the real id arrives.
  const isTemp = task.id.startsWith('temp-');

  const onStatusChange = (status: TaskStatus) => {
    if (status === task.status || isTemp) return;
    update.mutate({ id: task.id, body: { status } });
  };

  const onPriorityChange = (priority: TaskPriority) => {
    if (priority === task.priority || isTemp) return;
    update.mutate({ id: task.id, body: { priority } });
  };

  const onDelete = () => {
    if (isTemp) return;
    del.mutate(task.id);
  };

  // Animates the in-card swap between view-mode and edit-form.
  const [cardRef] = useAutoAnimate<HTMLDivElement>(ANIMATE_OPTS);

  return (
    <Card
      className={cn(
        'p-4 transition-[opacity,box-shadow] duration-150',
        isTemp && 'opacity-70',
      )}
    >
      <div ref={cardRef}>
        {editing ? (
          <EditTaskForm task={task} onDone={() => setEditing(false)} />
        ) : (
          <div className="flex flex-col gap-3">
            <div className="flex items-start justify-between gap-3">
              <div className="min-w-0">
                <h2
                  className={cn(
                    'truncate text-base font-medium transition-colors duration-200',
                    task.status === 'done' && 'text-muted-foreground line-through',
                  )}
                  title={task.title}
                >
                  {task.title}
                </h2>
                {task.description && (
                  <p className="mt-1 whitespace-pre-wrap text-sm text-muted-foreground">
                    {task.description}
                  </p>
                )}
              </div>
              <div className="flex shrink-0 items-center gap-1">
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => setEditing(true)}
                  aria-label="Редактировать"
                  title="Редактировать"
                  disabled={isTemp}
                >
                  <Pencil className="h-4 w-4" aria-hidden="true" />
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={onDelete}
                  aria-label="Удалить"
                  title="Удалить"
                  disabled={isTemp || del.isPending}
                >
                  <Trash2 className="h-4 w-4 text-destructive" aria-hidden="true" />
                </Button>
              </div>
            </div>

            <div className="flex flex-wrap items-center gap-2 text-xs">
              <StatusSelect value={task.status} onChange={onStatusChange} />
              <PrioritySelect value={task.priority} onChange={onPriorityChange} />
              {task.due_date && (
                <span className="inline-flex items-center gap-1 rounded-full bg-muted px-2 py-1 text-muted-foreground transition-colors duration-200">
                  <CalendarDays className="h-3 w-3" aria-hidden="true" />
                  до {formatDueDate(task.due_date)}
                </span>
              )}
            </div>
          </div>
        )}
      </div>
    </Card>
  );
}

function StatusSelect({
  value,
  onChange,
}: {
  value: TaskStatus;
  onChange: (s: TaskStatus) => void;
}) {
  return (
    <label className="inline-flex items-center gap-1 text-muted-foreground">
      <span className="sr-only">Статус</span>
      <select
        value={value}
        onChange={(e) => onChange(e.target.value as TaskStatus)}
        className={cn(
          'h-7 rounded-md border border-input bg-background px-2 text-xs font-medium',
          'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring',
        )}
      >
        <option value="todo">{STATUS_LABEL.todo}</option>
        <option value="in_progress">{STATUS_LABEL.in_progress}</option>
        <option value="done">{STATUS_LABEL.done}</option>
      </select>
    </label>
  );
}

function PrioritySelect({
  value,
  onChange,
}: {
  value: TaskPriority;
  onChange: (p: TaskPriority) => void;
}) {
  return (
    <label
      className={cn(
        'inline-flex items-center rounded-full transition-colors duration-200',
        PRIORITY_TONE[value],
      )}
    >
      <span className="sr-only">Приоритет</span>
      <select
        value={value}
        onChange={(e) => onChange(e.target.value as TaskPriority)}
        className="h-7 cursor-pointer rounded-full bg-transparent px-2 text-xs font-medium focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
      >
        <option value="low">{PRIORITY_LABEL.low}</option>
        <option value="medium">{PRIORITY_LABEL.medium}</option>
        <option value="high">{PRIORITY_LABEL.high}</option>
      </select>
    </label>
  );
}

// ─── Edit form ─────────────────────────────────────────────────────────

function EditTaskForm({ task, onDone }: { task: Task; onDone: () => void }) {
  const update = useUpdateTask({ onSuccess: () => onDone() });

  const {
    register,
    handleSubmit,
    formState: { errors, isSubmitting },
  } = useForm<EditValues>({
    resolver: zodResolver(editSchema),
    defaultValues: {
      title: task.title,
      description: task.description ?? '',
      priority: task.priority,
      due_date: isoToDateInput(task.due_date),
    },
  });

  const onSubmit = handleSubmit(async (values) => {
    await update.mutateAsync({
      id: task.id,
      body: {
        title: values.title,
        // empty string clears description (per backend PATCH semantics)
        description: values.description ?? '',
        priority: values.priority,
        due_date: dueDateToIso(values.due_date),
      },
    });
  });

  const serverError = update.error instanceof ApiError ? update.error.message : null;

  return (
    <form onSubmit={onSubmit} className="space-y-3" noValidate>
      <div>
        <label className="mb-1 block text-sm font-medium" htmlFor={`edit-title-${task.id}`}>
          Название
        </label>
        <Input
          id={`edit-title-${task.id}`}
          aria-invalid={!!errors.title}
          error={errors.title?.message}
          {...register('title')}
        />
      </div>

      <div>
        <label
          className="mb-1 block text-sm font-medium"
          htmlFor={`edit-description-${task.id}`}
        >
          Описание
        </label>
        <textarea
          id={`edit-description-${task.id}`}
          rows={3}
          className={cn(
            'flex w-full rounded-md border border-input bg-background px-3 py-2 text-sm',
            'placeholder:text-muted-foreground',
            'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring',
          )}
          {...register('description')}
        />
      </div>

      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <div>
          <label
            className="mb-1 block text-sm font-medium"
            htmlFor={`edit-priority-${task.id}`}
          >
            Приоритет
          </label>
          <select
            id={`edit-priority-${task.id}`}
            className={cn(
              'flex h-10 w-full rounded-md border border-input bg-background px-3 text-sm',
              'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring',
            )}
            {...register('priority')}
          >
            <option value="low">{PRIORITY_LABEL.low}</option>
            <option value="medium">{PRIORITY_LABEL.medium}</option>
            <option value="high">{PRIORITY_LABEL.high}</option>
          </select>
        </div>
        <div>
          <label className="mb-1 block text-sm font-medium" htmlFor={`edit-due-${task.id}`}>
            Срок
          </label>
          <Input id={`edit-due-${task.id}`} type="date" {...register('due_date')} />
        </div>
      </div>

      {serverError && (
        <p className="text-sm text-destructive" role="alert">
          {serverError}
        </p>
      )}

      <div className="flex justify-end gap-2 pt-1">
        <Button type="button" variant="ghost" onClick={onDone}>
          Отмена
        </Button>
        <Button type="submit" disabled={isSubmitting}>
          {isSubmitting ? 'Сохраняем…' : 'Сохранить'}
        </Button>
      </div>
    </form>
  );
}

// ─── Skeleton + empty ──────────────────────────────────────────────────

function TaskListSkeleton() {
  return (
    <div className="space-y-3">
      {Array.from({ length: 3 }).map((_, i) => (
        <Card key={i} className="p-4">
          <Skeleton className="h-5 w-1/3" />
          <Skeleton className="mt-3 h-4 w-2/3" />
          <div className="mt-4 flex gap-2">
            <Skeleton className="h-6 w-24" />
            <Skeleton className="h-6 w-24" />
          </div>
        </Card>
      ))}
    </div>
  );
}

function EmptyState() {
  return (
    <Card className="flex flex-col items-center gap-3 px-6 py-12 text-center">
      <div className="flex h-12 w-12 items-center justify-center rounded-full bg-secondary">
        <ClipboardList
          className="h-5 w-5 text-secondary-foreground"
          aria-hidden="true"
        />
      </div>
      <p className="text-sm font-medium">Здесь пока пусто</p>
      <p className="text-sm text-muted-foreground">
        Создайте первую задачу, чтобы начать вести backlog.
      </p>
      <div className="mt-2 flex items-center gap-2 text-xs text-muted-foreground">
        <CircleDashed className="h-3 w-3" aria-hidden="true" />
        К выполнению
        <CheckCircle2 className="ml-2 h-3 w-3" aria-hidden="true" />
        Готово
      </div>
    </Card>
  );
}
