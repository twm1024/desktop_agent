import { ReactNode } from "react";
import { Link, useLocation } from "react-router-dom";

interface LayoutProps {
  children: ReactNode;
  theme: "light" | "dark";
  toggleTheme: () => void;
}

export function Layout({ children, theme, toggleTheme }: LayoutProps) {
  const location = useLocation();

  const isActive = (path: string) => location.pathname === path;

  return (
    <div className="min-h-screen bg-background text-foreground">
      <header className="border-b border-border bg-card">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
          <div className="flex h-16 items-center justify-between">
            <div className="flex items-center gap-8">
              <h1 className="text-xl font-bold">Desktop Agent</h1>
              <nav className="flex gap-4">
                <Link
                  to="/"
                  className={`px-3 py-2 text-sm font-medium transition-colors hover:text-primary ${
                    isActive("/") ? "text-primary" : "text-muted-foreground"
                  }`}
                >
                  仪表板
                </Link>
                <Link
                  to="/skills"
                  className={`px-3 py-2 text-sm font-medium transition-colors hover:text-primary ${
                    isActive("/skills") ? "text-primary" : "text-muted-foreground"
                  }`}
                >
                  技能管理
                </Link>
                <Link
                  to="/market"
                  className={`px-3 py-2 text-sm font-medium transition-colors hover:text-primary ${
                    isActive("/market") ? "text-primary" : "text-muted-foreground"
                  }`}
                >
                  技能市场
                </Link>
                <Link
                  to="/monitor"
                  className={`px-3 py-2 text-sm font-medium transition-colors hover:text-primary ${
                    isActive("/monitor") ? "text-primary" : "text-muted-foreground"
                  }`}
                >
                  系统监控
                </Link>
                <Link
                  to="/settings"
                  className={`px-3 py-2 text-sm font-medium transition-colors hover:text-primary ${
                    isActive("/settings") ? "text-primary" : "text-muted-foreground"
                  }`}
                >
                  设置
                </Link>
              </nav>
            </div>
            <button
              onClick={toggleTheme}
              className="rounded-md p-2 hover:bg-accent hover:text-accent-foreground"
              aria-label="Toggle theme"
            >
              {theme === "dark" ? "☀️" : "🌙"}
            </button>
          </div>
        </div>
      </header>
      <main className="mx-auto max-w-7xl px-4 py-6 sm:px-6 lg:px-8">
        {children}
      </main>
    </div>
  );
}
