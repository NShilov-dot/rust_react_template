import { useEffect, useState } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';

/**
 * Read the `?oauth_error=<code>` query param set by the backend's
 * /auth/google/* error redirect, translate it to a human message, and
 * clean the URL so a refresh doesn't show the banner forever.
 *
 * Codes (kept stable on the backend, see api/handlers/google.rs):
 *   denied     — user clicked "Cancel" at Google
 *   expired    — state/PKCE expired; user took too long
 *   unverified — Google didn't verify the email (we refuse to auto-link)
 *   network    — Google/network glitch
 *   internal   — anything else
 *   bad_request — missing code/state in the callback
 */
const MESSAGES: Record<string, string> = {
  denied: 'Вы отменили вход через Google',
  expired: 'Сессия входа истекла. Попробуйте ещё раз.',
  unverified:
    'Google не подтвердил, что email принадлежит вам. Войдите по паролю и привяжите Google в настройках.',
  network: 'Не удалось связаться с Google. Попробуйте позже.',
  internal: 'Не удалось войти через Google. Попробуйте ещё раз.',
  bad_request: 'Некорректный ответ от Google. Попробуйте ещё раз.',
};

export function useOAuthError(): string | null {
  const location = useLocation();
  const navigate = useNavigate();
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    const params = new URLSearchParams(location.search);
    const code = params.get('oauth_error');
    if (!code) return;

    setMessage(MESSAGES[code] ?? MESSAGES.internal!);

    // Strip the param so a refresh doesn't keep showing the banner.
    params.delete('oauth_error');
    const next = params.toString();
    navigate(
      { pathname: location.pathname, search: next ? `?${next}` : '' },
      { replace: true, state: location.state },
    );
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return message;
}
