export function Footer() {
  const year = new Date().getFullYear();
  return (
    <footer className="border-t border-border">
      <div className="mx-auto flex max-w-5xl items-center justify-between gap-4 px-4 py-4 text-xs text-muted-foreground">
        <span>© {year} Rust+React</span>
        <span>
          axum · sqlx · redis · react · vite
        </span>
      </div>
    </footer>
  );
}
