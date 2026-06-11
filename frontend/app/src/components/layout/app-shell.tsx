import { Outlet } from 'react-router-dom';

import { Footer } from '@/components/layout/footer';
import { Header } from '@/components/layout/header';
import { Sidebar, SidebarProvider } from '@/components/layout/sidebar';

/** Layout for authenticated routes: header + sidebar + main + footer. */
export function AppShell() {
  return (
    <SidebarProvider>
      <div className="flex min-h-screen flex-col bg-background">
        <Header />
        <div className="flex flex-1">
          <Sidebar />
          <div className="flex min-w-0 flex-1 flex-col">
            <main id="main" className="mx-auto w-full max-w-5xl flex-1 px-4 py-6">
              <Outlet />
            </main>
            <Footer />
          </div>
        </div>
      </div>
    </SidebarProvider>
  );
}
