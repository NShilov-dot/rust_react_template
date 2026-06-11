export interface User {
  id: string;
  email: string;
  name: string;
  created_at: string;
  updated_at: string;
}

/** Response from `/auth/login` and `/auth/register`. */
export interface AuthResponse {
  user: User;
  access_token: string;
  access_expires_at: string;
}

/** Response from `/auth/refresh`. */
export interface AccessResponse {
  access_token: string;
  access_expires_at: string;
}

export interface ApiErrorBody {
  error: string;
}
