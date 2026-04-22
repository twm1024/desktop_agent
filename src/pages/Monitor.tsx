// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

import React, { useState, useEffect } from 'react';

interface SystemMetrics {
  cpu_usage: number;
  memory_total: number;
  memory_used: number;
  memory_percent: number;
  disk_total: number;
  disk_used: number;
  disk_percent: number;
  uptime: number;
  process_count: number;
  active_tasks: number;
}

interface PerformanceMetrics {
  total_operations: number;
  total_errors: number;
  error_rate: number;
  operation_types: number;
  active_operations: number;
  uptime_secs: number;
}

interface CacheStats {
  total: number;
  active: number;
  expired: number;
  max_entries: number;
}

const Monitor: React.FC = () => {
  const [systemMetrics, setSystemMetrics] = useState<SystemMetrics>({
    cpu_usage: 0,
    memory_total: 0,
    memory_used: 0,
    memory_percent: 0,
    disk_total: 0,
    disk_used: 0,
    disk_percent: 0,
    uptime: 0,
    process_count: 0,
    active_tasks: 0,
  });

  const [perfMetrics, setPerfMetrics] = useState<PerformanceMetrics>({
    total_operations: 0,
    total_errors: 0,
    error_rate: 0,
    operation_types: 0,
    active_operations: 0,
    uptime_secs: 0,
  });

  const [cacheStats, setCacheStats] = useState<Record<string, CacheStats>>({});

  useEffect(() => {
    // Simulated metrics - in production these would come from Tauri backend
    const interval = setInterval(() => {
      setSystemMetrics((prev) => ({
        ...prev,
        cpu_usage: Math.random() * 30 + 10,
        memory_percent: Math.random() * 20 + 45,
        uptime: prev.uptime + 1,
        active_tasks: Math.floor(Math.random() * 5),
      }));
    }, 2000);

    // Initial perf metrics
    setPerfMetrics({
      total_operations: 1247,
      total_errors: 23,
      error_rate: 1.84,
      operation_types: 8,
      active_operations: 3,
      uptime_secs: 86400,
    });

    setCacheStats({
      skill_manifests: { total: 12, active: 12, expired: 0, max_entries: 200 },
      platform_tokens: { total: 3, active: 3, expired: 0, max_entries: 20 },
      user_info: { total: 45, active: 42, expired: 3, max_entries: 500 },
      system_info: { total: 5, active: 5, expired: 0, max_entries: 10 },
      general: { total: 128, active: 115, expired: 13, max_entries: 1000 },
    });

    return () => clearInterval(interval);
  }, []);

  const formatUptime = (secs: number): string => {
    const h = Math.floor(secs / 3600);
    const m = Math.floor((secs % 3600) / 60);
    const s = secs % 60;
    return `${h}h ${m}m ${s}s`;
  };

  const ProgressBar: React.FC<{ value: number; color: string }> = ({ value, color }) => (
    <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2.5">
      <div
        className={`${color} h-2.5 rounded-full transition-all duration-500`}
        style={{ width: `${Math.min(value, 100)}%` }}
      />
    </div>
  );

  return (
    <div className="p-6 max-w-6xl mx-auto">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-gray-900 dark:text-white">系统监控</h1>
        <span className="text-sm text-gray-500 dark:text-gray-400">
          运行时间: {formatUptime(systemMetrics.uptime)}
        </span>
      </div>

      {/* System Resources */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
        <div className="p-4 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium text-gray-700 dark:text-gray-300">CPU 使用率</span>
            <span className="text-sm text-gray-500 dark:text-gray-400">{systemMetrics.cpu_usage.toFixed(1)}%</span>
          </div>
          <ProgressBar value={systemMetrics.cpu_usage} color="bg-blue-500" />
        </div>

        <div className="p-4 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium text-gray-700 dark:text-gray-300">内存使用率</span>
            <span className="text-sm text-gray-500 dark:text-gray-400">{systemMetrics.memory_percent.toFixed(1)}%</span>
          </div>
          <ProgressBar value={systemMetrics.memory_percent} color="bg-green-500" />
        </div>

        <div className="p-4 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium text-gray-700 dark:text-gray-300">磁盘使用率</span>
            <span className="text-sm text-gray-500 dark:text-gray-400">--</span>
          </div>
          <ProgressBar value={systemMetrics.disk_percent} color="bg-purple-500" />
        </div>
      </div>

      {/* Performance Overview */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-6">
        <div className="p-4 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg text-center">
          <p className="text-2xl font-bold text-blue-600 dark:text-blue-400">
            {perfMetrics.total_operations}
          </p>
          <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">总操作数</p>
        </div>
        <div className="p-4 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg text-center">
          <p className="text-2xl font-bold text-red-600 dark:text-red-400">
            {perfMetrics.total_errors}
          </p>
          <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">错误数</p>
        </div>
        <div className="p-4 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg text-center">
          <p className="text-2xl font-bold text-yellow-600 dark:text-yellow-400">
            {perfMetrics.error_rate.toFixed(2)}%
          </p>
          <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">错误率</p>
        </div>
        <div className="p-4 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg text-center">
          <p className="text-2xl font-bold text-green-600 dark:text-green-400">
            {perfMetrics.active_operations}
          </p>
          <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">活跃操作</p>
        </div>
      </div>

      {/* Cache Stats */}
      <div className="mb-6">
        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">缓存状态</h2>
        <div className="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
          <table className="w-full text-sm">
            <thead className="bg-gray-50 dark:bg-gray-700">
              <tr>
                <th className="px-4 py-3 text-left text-gray-600 dark:text-gray-300">缓存名称</th>
                <th className="px-4 py-3 text-center text-gray-600 dark:text-gray-300">活跃</th>
                <th className="px-4 py-3 text-center text-gray-600 dark:text-gray-300">过期</th>
                <th className="px-4 py-3 text-center text-gray-600 dark:text-gray-300">总计</th>
                <th className="px-4 py-3 text-center text-gray-600 dark:text-gray-300">容量</th>
                <th className="px-4 py-3 text-center text-gray-600 dark:text-gray-300">使用率</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-200 dark:divide-gray-600">
              {Object.entries(cacheStats).map(([name, stats]) => (
                <tr key={name}>
                  <td className="px-4 py-2 text-gray-900 dark:text-white">{name}</td>
                  <td className="px-4 py-2 text-center text-green-600 dark:text-green-400">{stats.active}</td>
                  <td className="px-4 py-2 text-center text-gray-500 dark:text-gray-400">{stats.expired}</td>
                  <td className="px-4 py-2 text-center text-gray-900 dark:text-white">{stats.total}</td>
                  <td className="px-4 py-2 text-center text-gray-500 dark:text-gray-400">{stats.max_entries}</td>
                  <td className="px-4 py-2 text-center">
                    <div className="flex items-center justify-center gap-2">
                      <div className="w-16 bg-gray-200 dark:bg-gray-600 rounded-full h-1.5">
                        <div
                          className="bg-blue-500 h-1.5 rounded-full"
                          style={{ width: `${(stats.total / stats.max_entries) * 100}%` }}
                        />
                      </div>
                      <span className="text-xs text-gray-500 dark:text-gray-400">
                        {((stats.total / stats.max_entries) * 100).toFixed(0)}%
                      </span>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Security Audit Summary */}
      <div>
        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">安全审计</h2>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div className="p-4 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg">
            <div className="flex items-center gap-2 mb-2">
              <div className="w-2 h-2 bg-green-500 rounded-full" />
              <span className="text-sm font-medium text-gray-700 dark:text-gray-300">认证事件</span>
            </div>
            <p className="text-xs text-gray-500 dark:text-gray-400">最近24h: 156 次登录</p>
            <p className="text-xs text-green-600 dark:text-green-400">无异常</p>
          </div>

          <div className="p-4 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg">
            <div className="flex items-center gap-2 mb-2">
              <div className="w-2 h-2 bg-green-500 rounded-full" />
              <span className="text-sm font-medium text-gray-700 dark:text-gray-300">技能执行</span>
            </div>
            <p className="text-xs text-gray-500 dark:text-gray-400">最近24h: 89 次执行</p>
            <p className="text-xs text-green-600 dark:text-green-400">全部成功</p>
          </div>

          <div className="p-4 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg">
            <div className="flex items-center gap-2 mb-2">
              <div className="w-2 h-2 bg-yellow-500 rounded-full" />
              <span className="text-sm font-medium text-gray-700 dark:text-gray-300">速率限制</span>
            </div>
            <p className="text-xs text-gray-500 dark:text-gray-400">最近24h: 3 次触发</p>
            <p className="text-xs text-yellow-600 dark:text-yellow-400">需关注</p>
          </div>
        </div>
      </div>
    </div>
  );
};

export default Monitor;
