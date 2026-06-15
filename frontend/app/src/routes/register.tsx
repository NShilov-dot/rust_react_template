import { useState } from 'react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { Link, useNavigate } from 'react-router-dom';
import { z } from 'zod';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Card } from '@/components/ui/card';
import { GoogleButton, OAuthDivider } from '@/components/google-button';
import { useAuth } from '@/hooks/use-auth';
import { useOAuthError } from '@/hooks/use-oauth-error';
import { ApiError } from '@/lib/api';

const schema = z.object({
  email: z.string().email('Введите корректный email'),
  name: z.string().min(1, 'Имя обязательно').max(100),
  password: z
    .string()
    .min(8, 'Минимум 8 символов')
    .max(256, 'Максимум 256 символов'),
});

type FormValues = z.infer<typeof schema>;

export default function RegisterPage() {
  const { register: registerUser } = useAuth();
  const navigate = useNavigate();
  const [serverError, setServerError] = useState<string | null>(null);
  const oauthError = useOAuthError();

  const {
    register,
    handleSubmit,
    formState: { errors, isSubmitting },
  } = useForm<FormValues>({ resolver: zodResolver(schema) });

  const onSubmit = handleSubmit(async ({ email, name, password }) => {
    setServerError(null);
    try {
      await registerUser(email, name, password);
      navigate('/dashboard', { replace: true });
    } catch (e) {
      const msg =
        e instanceof ApiError && e.status === 409
          ? 'Этот email уже занят'
          : e instanceof ApiError && e.status === 429
            ? 'Слишком много попыток. Попробуйте через минуту.'
            : e instanceof Error
              ? e.message
              : 'Что-то пошло не так';
      setServerError(msg);
    }
  });

  return (
    <Card className="w-full max-w-md p-8 shadow-sm">
      <div className="space-y-1">
        <h1 className="text-2xl font-semibold tracking-tight">Создать аккаунт</h1>
        <p className="text-sm text-muted-foreground">
          Несколько полей — и вы внутри
        </p>
      </div>

      {oauthError && (
        <div
          className="mt-6 rounded-md border border-destructive/50 bg-destructive/10 px-3 py-2 text-sm text-destructive"
          role="alert"
        >
          {oauthError}
        </div>
      )}

      <div className="mt-6 space-y-3">
        <GoogleButton label="Зарегистрироваться через Google" />
        <OAuthDivider />
      </div>

      <form onSubmit={onSubmit} className="mt-4 space-y-4" noValidate>
        <Field label="Email" htmlFor="email" error={errors.email?.message}>
          <Input
            id="email"
            type="email"
            autoComplete="email"
            placeholder="you@example.com"
            aria-invalid={!!errors.email}
            {...register('email')}
          />
        </Field>

        <Field label="Имя" htmlFor="name" error={errors.name?.message}>
          <Input
            id="name"
            type="text"
            autoComplete="name"
            placeholder="Ваше имя"
            aria-invalid={!!errors.name}
            {...register('name')}
          />
        </Field>

        <Field label="Пароль" htmlFor="password" error={errors.password?.message}>
          <Input
            id="password"
            type="password"
            autoComplete="new-password"
            placeholder="Минимум 8 символов"
            aria-invalid={!!errors.password}
            {...register('password')}
          />
        </Field>

        {serverError && (
          <div
            className="rounded-md border border-destructive/50 bg-destructive/10 px-3 py-2 text-sm text-destructive"
            role="alert"
          >
            {serverError}
          </div>
        )}

        <Button type="submit" disabled={isSubmitting} className="w-full">
          {isSubmitting ? 'Создаём…' : 'Создать аккаунт'}
        </Button>
      </form>

      <p className="mt-6 text-center text-sm text-muted-foreground">
        Уже есть аккаунт?{' '}
        <Link
          to="/login"
          className="font-medium text-foreground underline-offset-4 hover:underline"
        >
          Войти
        </Link>
      </p>
    </Card>
  );
}

function Field({
  label,
  htmlFor,
  error,
  children,
}: {
  label: string;
  htmlFor: string;
  error?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="space-y-1.5">
      <label htmlFor={htmlFor} className="block text-sm font-medium">
        {label}
      </label>
      {children}
      {error && (
        <p className="text-xs text-destructive" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}
