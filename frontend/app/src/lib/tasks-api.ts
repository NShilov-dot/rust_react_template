import {
  useInfiniteQuery,
  useMutation,
  useQueryClient,
  type InfiniteData,
  type QueryClient,
  type UseMutationOptions,
} from '@tanstack/react-query';

import { api } from '@/lib/api';
import type {
  CreateTaskBody,
  Task,
  TasksPage,
  TaskStatus,
  UpdateTaskBody,
} from '@/types/tasks';

const TASKS_KEY = 'tasks';
const PAGE_SIZE = 30;

export interface TasksFilter {
  status?: TaskStatus;
}

function buildPath(filter: TasksFilter, cursor: string | null): string {
  const params = new URLSearchParams();
  if (filter.status) params.set('status', filter.status);
  if (cursor) params.set('cursor', cursor);
  params.set('limit', String(PAGE_SIZE));
  return `/tasks?${params.toString()}`;
}

/** Cursor-paginated infinite query — pages are loaded on demand as the user
 *  scrolls. Keyed by filter, so each status tab keeps its own scroll state. */
export function useTasks(filter: TasksFilter = {}) {
  return useInfiniteQuery({
    queryKey: [TASKS_KEY, filter],
    queryFn: ({ pageParam }) =>
      api.get<TasksPage>(buildPath(filter, pageParam)),
    initialPageParam: null as string | null,
    getNextPageParam: (lastPage) => lastPage.next_cursor,
  });
}

// ─── Cache helpers ─────────────────────────────────────────────────────
//
// Every mutation hook updates the cache directly (optimistic) and rolls back
// on error. We avoid `invalidateQueries` for routine edits — refetching every
// loaded page of every filter on each status click would be wasteful. The
// trade-off: we trust the local merge to be correct for steady-state edits.

type CacheEntry = [readonly unknown[], InfiniteData<TasksPage> | undefined];

function snapshotInfinite(qc: QueryClient): CacheEntry[] {
  return qc.getQueriesData<InfiniteData<TasksPage>>({ queryKey: [TASKS_KEY] });
}

function rollback(qc: QueryClient, snap: CacheEntry[]) {
  snap.forEach(([key, data]) => qc.setQueryData(key, data));
}

function filterOf(key: readonly unknown[]): TasksFilter {
  return (key[1] as TasksFilter | undefined) ?? {};
}

function eachCache(
  qc: QueryClient,
  fn: (
    key: readonly unknown[],
    data: InfiniteData<TasksPage>,
    filter: TasksFilter,
  ) => InfiniteData<TasksPage> | undefined,
) {
  const entries = qc.getQueriesData<InfiniteData<TasksPage>>({
    queryKey: [TASKS_KEY],
  });
  entries.forEach(([key, data]) => {
    if (!data) return;
    const next = fn(key, data, filterOf(key));
    if (next !== undefined) qc.setQueryData(key, next);
  });
}

// ─── Create ────────────────────────────────────────────────────────────

interface CreateCtx {
  snap: CacheEntry[];
  tempId: string;
}

export function useCreateTask(
  opts?: UseMutationOptions<Task, unknown, CreateTaskBody, CreateCtx>,
) {
  const qc = useQueryClient();
  return useMutation<Task, unknown, CreateTaskBody, CreateCtx>({
    mutationFn: (body) => api.post<Task>('/tasks', body),
    onMutate: async (body) => {
      await qc.cancelQueries({ queryKey: [TASKS_KEY] });
      const snap = snapshotInfinite(qc);
      const now = new Date().toISOString();
      const tempId = `temp-${now}-${Math.random().toString(36).slice(2, 8)}`;
      const temp: Task = {
        id: tempId,
        owner_id: '',
        title: body.title,
        description: body.description?.trim() ? body.description : null,
        status: 'todo',
        priority: body.priority ?? 'medium',
        due_date: body.due_date ?? null,
        created_at: now,
        updated_at: now,
      };
      // Insert into every cache whose filter accepts a `todo` task.
      eachCache(qc, (_key, data, filter) => {
        if (filter.status && filter.status !== temp.status) return undefined;
        const [first, ...rest] = data.pages;
        if (!first) return undefined;
        return {
          ...data,
          pages: [{ ...first, tasks: [temp, ...first.tasks] }, ...rest],
        };
      });
      return { snap, tempId };
    },
    onSuccess: (real, _vars, ctx) => {
      if (!ctx) return;
      // Replace the temp row with the server-issued one (real id + owner_id).
      eachCache(qc, (_key, data) => ({
        ...data,
        pages: data.pages.map((p) => ({
          ...p,
          tasks: p.tasks.map((t) => (t.id === ctx.tempId ? real : t)),
        })),
      }));
    },
    onError: (_e, _v, ctx) => ctx && rollback(qc, ctx.snap),
    ...opts,
  });
}

// ─── Update ────────────────────────────────────────────────────────────

interface UpdateVars {
  id: string;
  body: UpdateTaskBody;
}

interface UpdateCtx {
  snap: CacheEntry[];
}

export function useUpdateTask(
  opts?: UseMutationOptions<Task, unknown, UpdateVars, UpdateCtx>,
) {
  const qc = useQueryClient();
  return useMutation<Task, unknown, UpdateVars, UpdateCtx>({
    mutationFn: ({ id, body }) => api.patch<Task>(`/tasks/${id}`, body),
    onMutate: async ({ id, body }) => {
      await qc.cancelQueries({ queryKey: [TASKS_KEY] });
      const snap = snapshotInfinite(qc);
      const now = new Date().toISOString();
      eachCache(qc, (_key, data, filter) => {
        const pages = data.pages.map((page) => {
          const tasks = page.tasks
            .map((t) => {
              if (t.id !== id) return t;
              return {
                ...t,
                ...(body.title !== undefined && { title: body.title }),
                // Backend semantics: "" clears the description.
                ...(body.description !== undefined && {
                  description:
                    body.description === '' ? null : body.description,
                }),
                ...(body.status !== undefined && { status: body.status }),
                ...(body.priority !== undefined && { priority: body.priority }),
                ...(body.due_date !== undefined && { due_date: body.due_date }),
                updated_at: now,
              };
            })
            // If the updated status no longer matches this cache's filter,
            // drop the task so the user sees it leave the list immediately.
            .filter((t) => !filter.status || t.status === filter.status);
          return { ...page, tasks };
        });
        return { ...data, pages };
      });
      return { snap };
    },
    onSuccess: (real) => {
      // Reconcile with the server's authoritative copy (fresh `updated_at`).
      eachCache(qc, (_key, data, filter) => ({
        ...data,
        pages: data.pages.map((p) => ({
          ...p,
          tasks: p.tasks
            .map((t) => (t.id === real.id ? real : t))
            .filter((t) => !filter.status || t.status === filter.status),
        })),
      }));
    },
    onError: (_e, _v, ctx) => ctx && rollback(qc, ctx.snap),
    ...opts,
  });
}

// ─── Delete ────────────────────────────────────────────────────────────

interface DeleteCtx {
  snap: CacheEntry[];
}

export function useDeleteTask(
  opts?: UseMutationOptions<void, unknown, string, DeleteCtx>,
) {
  const qc = useQueryClient();
  return useMutation<void, unknown, string, DeleteCtx>({
    mutationFn: (id) => api.delete<void>(`/tasks/${id}`),
    onMutate: async (id) => {
      await qc.cancelQueries({ queryKey: [TASKS_KEY] });
      const snap = snapshotInfinite(qc);
      eachCache(qc, (_key, data) => ({
        ...data,
        pages: data.pages.map((p) => ({
          ...p,
          tasks: p.tasks.filter((t) => t.id !== id),
        })),
      }));
      return { snap };
    },
    onError: (_e, _v, ctx) => ctx && rollback(qc, ctx.snap),
    ...opts,
  });
}
