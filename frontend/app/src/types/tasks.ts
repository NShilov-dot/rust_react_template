export type TaskStatus = 'todo' | 'in_progress' | 'done';
export type TaskPriority = 'low' | 'medium' | 'high';

export interface Task {
  id: string;
  owner_id: string;
  title: string;
  description: string | null;
  status: TaskStatus;
  priority: TaskPriority;
  due_date: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateTaskBody {
  title: string;
  description?: string;
  priority?: TaskPriority;
  due_date?: string;
}

/** PATCH body — omitted fields stay unchanged. `description: ""` clears it. */
export interface UpdateTaskBody {
  title?: string;
  description?: string;
  status?: TaskStatus;
  priority?: TaskPriority;
  due_date?: string;
}

/** Response shape from `GET /tasks` — one page of cursor-based pagination. */
export interface TasksPage {
  tasks: Task[];
  /** Opaque cursor for the next page; `null` when this is the last page. */
  next_cursor: string | null;
}
