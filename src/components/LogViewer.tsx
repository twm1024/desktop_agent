import { useEffect, useState } from "react";

interface LogEntry {
  id: number;
  timestamp: number;
  level: "debug" | "info" | "warn" | "error";
  message: string;
  module?: string;
  userId?: string;
  sessionId?: string;
}

interface LogViewerProps {
  autoRefresh?: boolean;
  maxEntries?: number;
  filters?: {
    level?: string;
    module?: string;
    userId?: string;
    search?: string;
  };
}

export function LogViewer({ autoRefresh = true, maxEntries: _maxEntries = 1000, filters }: LogViewerProps) {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [expanded, setExpanded] = useState<Set<number>>(new Set());
  const [searchTerm, setSearchTerm] = useState(filters?.search || "");
  const [levelFilter, setLevelFilter] = useState<string>(filters?.level || "all");

  useEffect(() => {
    fetchLogs();

    if (autoRefresh) {
      const interval = setInterval(fetchLogs, 5000);
      return () => clearInterval(interval);
    }
  }, [autoRefresh, filters]);

  useEffect(() => {
    if (searchTerm || levelFilter !== "all") {
      filterLogs();
    }
  }, [searchTerm, levelFilter]);

  async function fetchLogs() {
    try {
      // This will be connected to Tauri command
      // const data = await invoke<LogEntry[]>("get_logs", { limit: maxEntries });
      // setLogs(data);
      setLoading(false);
    } catch (err) {
      console.error("Failed to fetch logs:", err);
      setLoading(false);
    }
  }

  function filterLogs() {
    // Client-side filtering
    let filtered = logs;

    if (levelFilter !== "all") {
      filtered = filtered.filter((log) => log.level === levelFilter);
    }

    if (searchTerm) {
      const lower = searchTerm.toLowerCase();
      filtered = filtered.filter(
        (log) =>
          log.message.toLowerCase().includes(lower) ||
          log.module?.toLowerCase().includes(lower)
      );
    }

    return filtered;
  }

  function toggleExpand(id: number) {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  }

  function clearLogs() {
    // Clear displayed logs
    setLogs([]);
  }

  function exportLogs() {
    const data = JSON.stringify(logs, null, 2);
    const blob = new Blob([data], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `logs-${new Date().toISOString()}.json`;
    a.click();
    URL.revokeObjectURL(url);
  }

  function getLevelColor(level: string): string {
    switch (level) {
      case "error":
        return "text-red-600 bg-red-50 dark:text-red-400 dark:bg-red-900/20";
      case "warn":
        return "text-yellow-600 bg-yellow-50 dark:text-yellow-400 dark:bg-yellow-900/20";
      case "info":
        return "text-blue-600 bg-blue-50 dark:text-blue-400 dark:bg-blue-900/20";
      case "debug":
        return "text-gray-600 bg-gray-50 dark:text-gray-400 dark:bg-gray-900/20";
      default:
        return "text-gray-600 bg-gray-50 dark:text-gray-400 dark:bg-gray-900/20";
    }
  }

  function formatTimestamp(timestamp: number): string {
    return new Date(timestamp).toLocaleString("zh-CN", {
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  }

  const filteredLogs = searchTerm || levelFilter !== "all" ? filterLogs() : logs;

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b border-border">
        <h2 className="text-lg font-semibold">日志查看器</h2>
        <div className="flex gap-2">
          <button
            onClick={() => fetchLogs()}
            className="px-3 py-1.5 text-sm rounded-md border border-border bg-background hover:bg-accent"
          >
            刷新
          </button>
          <button
            onClick={clearLogs}
            className="px-3 py-1.5 text-sm rounded-md border border-border bg-background hover:bg-accent"
          >
            清空
          </button>
          <button
            onClick={exportLogs}
            className="px-3 py-1.5 text-sm rounded-md border border-border bg-background hover:bg-accent"
          >
            导出
          </button>
        </div>
      </div>

      {/* Filters */}
      <div className="flex items-center gap-4 p-4 border-b border-border bg-card">
        <div className="flex items-center gap-2">
          <label className="text-sm text-muted-foreground">级别:</label>
          <select
            value={levelFilter}
            onChange={(e) => setLevelFilter(e.target.value)}
            className="px-2 py-1 text-sm rounded-md border border-input bg-background"
          >
            <option value="all">全部</option>
            <option value="error">错误</option>
            <option value="warn">警告</option>
            <option value="info">信息</option>
            <option value="debug">调试</option>
          </select>
        </div>

        <div className="flex-1">
          <input
            type="text"
            placeholder="搜索日志..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="w-full px-3 py-1.5 text-sm rounded-md border border-input bg-background"
          />
        </div>

        <div className="text-sm text-muted-foreground">
          显示 {filteredLogs.length} 条
        </div>
      </div>

      {/* Log entries */}
      <div className="flex-1 overflow-y-auto p-4 space-y-2 bg-background">
        {loading ? (
          <div className="flex items-center justify-center h-full">
            <div className="text-muted-foreground">加载中...</div>
          </div>
        ) : filteredLogs.length === 0 ? (
          <div className="flex items-center justify-center h-full">
            <div className="text-center">
              <div className="text-4xl mb-4">📋</div>
              <p className="text-muted-foreground">暂无日志</p>
            </div>
          </div>
        ) : (
          filteredLogs.map((log) => (
            <div
              key={log.id}
              className={`p-3 rounded-lg border border-border font-mono text-sm cursor-pointer transition-colors hover:bg-accent ${
                expanded.has(log.id) ? "bg-accent" : "bg-card"
              }`}
              onClick={() => toggleExpand(log.id)}
            >
              <div className="flex items-start gap-3">
                <span className={`px-2 py-0.5 rounded text-xs font-medium ${getLevelColor(log.level)}`}>
                  {log.level.toUpperCase()}
                </span>
                <span className="text-muted-foreground text-xs whitespace-nowrap">
                  {formatTimestamp(log.timestamp)}
                </span>
                {log.module && (
                  <span className="text-muted-foreground text-xs">
                    [{log.module}]
                  </span>
                )}
                <span className="flex-1 break-all">
                  {log.message}
                </span>
              </div>

              {expanded.has(log.id) && (
                <div className="mt-3 pt-3 border-t border-border space-y-1 text-xs text-muted-foreground">
                  {log.userId && (
                    <div>
                      <span className="font-medium">用户ID:</span> {log.userId}
                    </div>
                  )}
                  {log.sessionId && (
                    <div>
                      <span className="font-medium">会话ID:</span> {log.sessionId}
                    </div>
                  )}
                  <div>
                    <span className="font-medium">时间戳:</span>{" "}
                    {new Date(log.timestamp).toISOString()}
                  </div>
                </div>
              )}
            </div>
          ))
        )}
      </div>
    </div>
  );
}

interface LogStatsProps {
  logs: LogEntry[];
}

export function LogStats({ logs }: LogStatsProps) {
  const stats = {
    total: logs.length,
    error: logs.filter((l) => l.level === "error").length,
    warn: logs.filter((l) => l.level === "warn").length,
    info: logs.filter((l) => l.level === "info").length,
    debug: logs.filter((l) => l.level === "debug").length,
  };

  return (
    <div className="grid grid-cols-5 gap-4 p-4 border-b border-border">
      <div className="text-center">
        <div className="text-2xl font-bold">{stats.total}</div>
        <div className="text-xs text-muted-foreground">总计</div>
      </div>
      <div className="text-center">
        <div className="text-2xl font-bold text-red-600">{stats.error}</div>
        <div className="text-xs text-muted-foreground">错误</div>
      </div>
      <div className="text-center">
        <div className="text-2xl font-bold text-yellow-600">{stats.warn}</div>
        <div className="text-xs text-muted-foreground">警告</div>
      </div>
      <div className="text-center">
        <div className="text-2xl font-bold text-blue-600">{stats.info}</div>
        <div className="text-xs text-muted-foreground">信息</div>
      </div>
      <div className="text-center">
        <div className="text-2xl font-bold text-gray-600">{stats.debug}</div>
        <div className="text-xs text-muted-foreground">调试</div>
      </div>
    </div>
  );
}
