import {
  useMutation,
  useQuery,
  useQueryClient,
  type QueryClient,
  type UseMutationOptions,
} from '@tanstack/react-query';

import { api } from '@/lib/api';
import type {
  CreateTaskBody,
  Task,
  TaskStatus,
  UpdateTaskBody,
} from '@/types/tasks';

const TASKS_KEY = 'tasks';

export interface TasksFilter {
  status?: TaskStatus;
}

function buildPath(filter: TasksFilter): string {
  const params = new URLSearchParams();
  if (filter.status) params.set('status', filter.status);
  params.set('limit', '100');
  const q = params.toString();
  return q ? `/tasks?${q}` : '/tasks';
}

export function useTasks(filter: TasksFilter = {}) {
  return useQuery({
    queryKey: [TASKS_KEY, filter],
    queryFn: () => api.get<Task[]>(buildPath(filter)),
  });
}

// ─── Optimistic-update helpers ─────────────────────────────────────────

/** Snapshot of every `tasks` list cache, used for rollback on error. */
type CacheSnapshot = ReturnType<QueryClient['getQueriesData']>;

async function snapshotTaskLists(qc: QueryClient): Promise<CacheSnapshot> {
  // Cancel in-flight fetches so they don't clobber our optimistic update on
  // arrival — TanStack's recipe for safe optimistic mutations.
  await qc.cancelQueries({ queryKey: [TASKS_KEY] });
  return qc.getQueriesData({ queryKey: [TASKS_KEY] });
}

function rollbackTaskLists(qc: QueryClient, snap: CacheSnapshot | undefined) {
  if (!snap) return;
  snap.forEach(([key, data]) => qc.setQueryData(key, data));
}

function updateAllTaskLists(qc: QueryClient, fn: (list: Task[]) => Task[]) {
  qc.setQueriesData<Task[]>({ queryKey: [TASKS_KEY] }, (old) => (old ? fn(old) : old));
}

// ─── Create ────────────────────────────────────────────────────────────

interface CreateCtx {
  snap: CacheSnapshot;
  tempId: string;
}

export function useCreateTask(
  opts?: UseMutationOptions<Task, unknown, CreateTaskBody, CreateCtx>,
) {
  const qc = useQueryClient();
  return useMutation<Task, unknown, CreateTaskBody, CreateCtx>({
    mutationFn: (body) => api.post<Task>('/tasks', body),
    onMutate: async (body) => {
      const snap = await snapshotTaskLists(qc);
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
      updateAllTaskLists(qc, (list) => [temp, ...list]);
      return { snap, tempId };
    },
    onError: (_e, _v, ctx) => rollbackTaskLists(qc, ctx?.snap),
    onSettled: () => qc.invalidateQueries({ queryKey: [TASKS_KEY] }),
    ...opts,
  });
}

// ─── Update ────────────────────────────────────────────────────────────

interface UpdateVars {
  id: string;
  body: UpdateTaskBody;
}

interface UpdateCtx {
  snap: CacheSnapshot;
}

export function useUpdateTask(
  opts?: UseMutationOptions<Task, unknown, UpdateVars, UpdateCtx>,
) {
  const qc = useQueryClient();
  return useMutation<Task, unknown, UpdateVars, UpdateCtx>({
    mutationFn: ({ id, body }) => api.patch<Task>(`/tasks/${id}`, body),
    onMutate: async ({ id, body }) => {
      const snap = await snapshotTaskLists(qc);
      const now = new Date().toISOString();
      updateAllTaskLists(qc, (list) =>
        list.map((t) => {
          if (t.id !== id) return t;
          return {
            ...t,
            ...(body.title !== undefined && { title: body.title }),
            // Backend semantics: "" clears the description.
            ...(body.description !== undefined && {
              description: body.description === '' ? null : body.description,
            }),
            ...(body.status !== undefined && { status: body.status }),
            ...(body.priority !== undefined && { priority: body.priority }),
            ...(body.due_date !== undefined && { due_date: body.due_date }),
            updated_at: now,
          };
        }),
      );
      return { snap };
    },
    onError: (_e, _v, ctx) => rollbackTaskLists(qc, ctx?.snap),
    onSettled: () => qc.invalidateQueries({ queryKey: [TASKS_KEY] }),
    ...opts,
  });
}

// ─── Delete ────────────────────────────────────────────────────────────

interface DeleteCtx {
  snap: CacheSnapshot;
}

export function useDeleteTask(
  opts?: UseMutationOptions<void, unknown, string, DeleteCtx>,
) {
  const qc = useQueryClient();
  return useMutation<void, unknown, string, DeleteCtx>({
    mutationFn: (id) => api.delete<void>(`/tasks/${id}`),
    onMutate: async (id) => {
      const snap = await snapshotTaskLists(qc);
      updateAllTaskLists(qc, (list) => list.filter((t) => t.id !== id));
      return { snap };
    },
    onError: (_e, _v, ctx) => rollbackTaskLists(qc, ctx?.snap),
    onSettled: () => qc.invalidateQueries({ queryKey: [TASKS_KEY] }),
    ...opts,
  });
}
