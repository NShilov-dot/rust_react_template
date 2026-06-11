import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { BrowserRouter } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

import { App } from '@/App';
import { SessionBootstrap } from '@/components/session-bootstrap';
import '@/index.css';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 1,
      refetchOnWindowFocus: false,
      staleTime: 30_000,
    },
  },
});

if (import.meta.env.DEV) {
  // Debug handle for the browser console / e2e tooling.
  (window as unknown as Record<string, unknown>).__queryClient = queryClient;
}

const rootEl = document.getElementById('root');
if (!rootEl) throw new Error('#root not found');

createRoot(rootEl).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <SessionBootstrap>
          <App />
        </SessionBootstrap>
      </BrowserRouter>
    </QueryClientProvider>
  </StrictMode>,
);
