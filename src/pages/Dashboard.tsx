import { useEffect, useState } from "react";

interface SystemInfo {
  os: string;
  os_version: string;
  cpu_brand: string;
  cpu_cores: number;
  total_memory: number;
  used_memory: number;
  uptime: number;
}

export function Dashboard() {
  const [systemInfo, setSystemInfo] = useState<SystemInfo | null>(null);
  void setSystemInfo;
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    async function fetchSystemInfo() {
      try {
        // This will be connected to Tauri command
        // const info = await invoke<SystemInfo>("get_system_info");
        // setSystemInfo(info);
        setError("Tauri backend not yet connected");
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to fetch system info");
      } finally {
        setLoading(false);
      }
    }

    fetchSystemInfo();
  }, []);

  const formatMemory = (bytes: number) => {
    const gb = bytes / (1024 * 1024 * 1024);
    return `${gb.toFixed(2)} GB`;
  };

  const formatUptime = (seconds: number) => {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    return `${hours}h ${minutes}m`;
  };

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-3xl font-bold tracking-tight">仪表板</h2>
        <p className="text-muted-foreground">
          Desktop Agent 控制面板
        </p>
      </div>

      {loading ? (
        <div className="flex items-center justify-center py-12">
          <div className="text-muted-foreground">加载中...</div>
        </div>
      ) : error ? (
        <div className="rounded-lg border border-destructive bg-destructive/10 p-4 text-destructive">
          {error}
        </div>
      ) : systemInfo ? (
        <div className="grid gap-6 md:grid-cols-2">
          <div className="rounded-lg border border-border bg-card p-6">
            <h3 className="text-lg font-semibold mb-4">系统信息</h3>
            <dl className="space-y-3">
              <div className="flex justify-between">
                <dt className="text-muted-foreground">操作系统</dt>
                <dd className="font-medium">{systemInfo.os} {systemInfo.os_version}</dd>
              </div>
              <div className="flex justify-between">
                <dt className="text-muted-foreground">CPU</dt>
                <dd className="font-medium">{systemInfo.cpu_brand}</dd>
              </div>
              <div className="flex justify-between">
                <dt className="text-muted-foreground">CPU 核心</dt>
                <dd className="font-medium">{systemInfo.cpu_cores}</dd>
              </div>
              <div className="flex justify-between">
                <dt className="text-muted-foreground">运行时间</dt>
                <dd className="font-medium">{formatUptime(systemInfo.uptime)}</dd>
              </div>
            </dl>
          </div>

          <div className="rounded-lg border border-border bg-card p-6">
            <h3 className="text-lg font-semibold mb-4">内存使用</h3>
            <div className="space-y-4">
              <div>
                <div className="flex justify-between text-sm mb-2">
                  <span className="text-muted-foreground">已使用</span>
                  <span className="font-medium">
                    {formatMemory(systemInfo.used_memory)} / {formatMemory(systemInfo.total_memory)}
                  </span>
                </div>
                <div className="h-2 w-full rounded-full bg-secondary">
                  <div
                    className="h-full rounded-full bg-primary"
                    style={{
                      width: `${(systemInfo.used_memory / systemInfo.total_memory) * 100}%`,
                    }}
                  />
                </div>
              </div>
            </div>
          </div>

          <div className="rounded-lg border border-border bg-card p-6 md:col-span-2">
            <h3 className="text-lg font-semibold mb-4">快速操作</h3>
            <div className="grid gap-4 sm:grid-cols-3">
              <button className="rounded-lg border border-border bg-background p-4 text-left transition-colors hover:bg-accent hover:text-accent-foreground">
                <div className="text-2xl mb-2">📁</div>
                <div className="font-medium">文件浏览</div>
                <div className="text-sm text-muted-foreground">浏览和管理文件</div>
              </button>
              <button className="rounded-lg border border-border bg-background p-4 text-left transition-colors hover:bg-accent hover:text-accent-foreground">
                <div className="text-2xl mb-2">⚡</div>
                <div className="font-medium">执行技能</div>
                <div className="text-sm text-muted-foreground">运行已安装的技能</div>
              </button>
              <button className="rounded-lg border border-border bg-background p-4 text-left transition-colors hover:bg-accent hover:text-accent-foreground">
                <div className="text-2xl mb-2">📊</div>
                <div className="font-medium">查看日志</div>
                <div className="text-sm text-muted-foreground">系统操作日志</div>
              </button>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );
}
