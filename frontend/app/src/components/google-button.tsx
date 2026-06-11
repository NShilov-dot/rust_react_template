import { cn } from '@/lib/utils';

interface Props {
  label: string;
  className?: string;
}

/**
 * "Continue with Google" — full-width OAuth entry button. Does a top-level
 * navigation to /api/auth/google/start; the backend builds the authorize
 * URL (PKCE + state) and 302s the browser to accounts.google.com. After
 * consent, the browser comes back via /api/auth/google/callback which
 * sets the refresh cookie and 302s to /dashboard.
 *
 * We use <a href> (not <button onClick>) so the browser treats this as
 * a same-document navigation — SessionBootstrap will run on the post-
 * callback page load, see the new cookie, refresh the session.
 */
export function GoogleButton({ label, className }: Props) {
  return (
    <a
      href="/api/auth/google/start"
      className={cn(
        'group/oauth relative inline-flex h-10 w-full items-center justify-center gap-2 overflow-hidden rounded-md border border-border bg-background px-4 text-sm font-medium text-foreground shadow-sm transition-all duration-200',
        'hover:border-foreground/20 hover:bg-accent/60 hover:shadow-md motion-reduce:transition-none',
        'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background',
        className,
      )}
    >
      <GoogleGlyph aria-hidden className="h-4 w-4 shrink-0" />
      <span>{label}</span>
    </a>
  );
}

/** Official Google "G" — flat SVG, no external font/asset. */
function GoogleGlyph(props: React.SVGProps<SVGSVGElement>) {
  return (
    <svg viewBox="0 0 48 48" {...props}>
      <path
        fill="#FFC107"
        d="M43.6 20.5H42V20.4H24v7.2h11.3c-1.5 4.2-5.5 7.2-11.3 7.2A11 11 0 1 1 24 12.4c2.8 0 5.4 1 7.4 2.7l5.1-5.1A18.4 18.4 0 1 0 24 42.4c10.3 0 19-7.5 19-18.4 0-1.2-.1-2.3-.4-3.5z"
      />
      <path
        fill="#FF3D00"
        d="m6.3 14.7 5.9 4.3a11 11 0 0 1 11.8-6.6c2.8 0 5.4 1 7.4 2.7l5.1-5.1A18.4 18.4 0 0 0 6.3 14.7z"
      />
      <path
        fill="#4CAF50"
        d="M24 42.4c5.1 0 9.7-2 13.1-5.1l-6-5.1a11 11 0 0 1-17-5.2l-5.9 4.6A18.4 18.4 0 0 0 24 42.4z"
      />
      <path
        fill="#1976D2"
        d="M43.6 20.5H42V20.4H24v7.2h11.3a11 11 0 0 1-3.8 5.6l6 5.1c-.4.4 6.6-4.8 6.6-14.4 0-1.2-.1-2.3-.4-3.5z"
      />
    </svg>
  );
}

/** Horizontal rule with "или" in the middle — used above/below the form. */
export function OAuthDivider() {
  return (
    <div className="flex items-center gap-3" aria-hidden="true">
      <span className="h-px flex-1 bg-border" />
      <span className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
        или
      </span>
      <span className="h-px flex-1 bg-border" />
    </div>
  );
}
