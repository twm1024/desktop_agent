import { useState } from "react";

interface ConfigSettings {
  webhook: {
    feishu: {
      enabled: boolean;
      secret: string;
    };
    wecom: {
      enabled: boolean;
      token: string;
      aes_key: string;
    };
    dingtalk: {
      enabled: boolean;
      secret: string;
    };
  };
  security: {
    rate_limit: {
      enabled: boolean;
      user_limit: number;
      ip_limit: number;
      window_minutes: number;
    };
    log_sanitization: {
      enabled: boolean;
    };
  };
  storage: {
    data_dir: string;
    skills_dir: string;
  };
}

export function Settings() {
  const [settings, setSettings] = useState<ConfigSettings>({
    webhook: {
      feishu: { enabled: false, secret: "" },
      wecom: { enabled: false, token: "", aes_key: "" },
      dingtalk: { enabled: false, secret: "" },
    },
    security: {
      rate_limit: {
        enabled: true,
        user_limit: 100,
        ip_limit: 1000,
        window_minutes: 60,
      },
      log_sanitization: {
        enabled: true,
      },
    },
    storage: {
      data_dir: "",
      skills_dir: "",
    },
  });
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);

  const handleSave = async () => {
    setSaving(true);
    setSaved(false);

    // Simulate save delay
    await new Promise((resolve) => setTimeout(resolve, 500));

    // TODO: Send to Tauri backend
    // await invoke("save_settings", { settings });

    setSaving(false);
    setSaved(true);

    setTimeout(() => setSaved(false), 2000);
  };

  const updateWebhook = (platform: keyof ConfigSettings["webhook"], field: string, value: any) => {
    setSettings((prev) => ({
      ...prev,
      webhook: {
        ...prev.webhook,
        [platform]: {
          ...prev.webhook[platform],
          [field]: value,
        },
      },
    }));
  };

  const updateSecurity = (section: keyof ConfigSettings["security"], field: string, value: any) => {
    setSettings((prev) => ({
      ...prev,
      security: {
        ...prev.security,
        [section]: {
          ...prev.security[section],
          [field]: value,
        },
      },
    }));
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-3xl font-bold tracking-tight">设置</h2>
          <p className="text-muted-foreground">
            配置 Desktop Agent
          </p>
        </div>
        <button
          onClick={handleSave}
          disabled={saving}
          className="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
        >
          {saving ? "保存中..." : saved ? "已保存" : "保存设置"}
        </button>
      </div>

      <div className="grid gap-6 lg:grid-cols-2">
        {/* Webhook Configuration */}
        <div className="rounded-lg border border-border bg-card p-6">
          <h3 className="text-lg font-semibold mb-4">Webhook 配置</h3>
          <div className="space-y-6">
            {/* Feishu */}
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <h4 className="font-medium">飞书</h4>
                <label className="relative inline-flex cursor-pointer items-center">
                  <input
                    type="checkbox"
                    checked={settings.webhook.feishu.enabled}
                    onChange={(e) =>
                      updateWebhook("feishu", "enabled", e.target.checked)
                    }
                    className="peer sr-only"
                  />
                  <div className="peer h-6 w-11 rounded-full bg-secondary after:absolute after:top-[2px] after:left-[2px] after:h-5 after:w-5 after:rounded-full after:border after:border-muted-foreground/20 after:bg-background after:transition-all after:content-[''] peer-checked:bg-primary peer-checked:after:translate-x-full" />
                </label>
              </div>
              {settings.webhook.feishu.enabled && (
                <input
                  type="password"
                  placeholder="加密密钥"
                  value={settings.webhook.feishu.secret}
                  onChange={(e) =>
                    updateWebhook("feishu", "secret", e.target.value)
                  }
                  className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
                />
              )}
            </div>

            {/* WeCom */}
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <h4 className="font-medium">企业微信</h4>
                <label className="relative inline-flex cursor-pointer items-center">
                  <input
                    type="checkbox"
                    checked={settings.webhook.wecom.enabled}
                    onChange={(e) =>
                      updateWebhook("wecom", "enabled", e.target.checked)
                    }
                    className="peer sr-only"
                  />
                  <div className="peer h-6 w-11 rounded-full bg-secondary after:absolute after:top-[2px] after:left-[2px] after:h-5 after:w-5 after:rounded-full after:border after:border-muted-foreground/20 after:bg-background after:transition-all after:content-[''] peer-checked:bg-primary peer-checked:after:translate-x-full" />
                </label>
              </div>
              {settings.webhook.wecom.enabled && (
                <div className="space-y-3">
                  <input
                    type="password"
                    placeholder="Token"
                    value={settings.webhook.wecom.token}
                    onChange={(e) =>
                      updateWebhook("wecom", "token", e.target.value)
                    }
                    className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
                  />
                  <input
                    type="password"
                    placeholder="AES Key"
                    value={settings.webhook.wecom.aes_key}
                    onChange={(e) =>
                      updateWebhook("wecom", "aes_key", e.target.value)
                    }
                    className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
                  />
                </div>
              )}
            </div>

            {/* DingTalk */}
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <h4 className="font-medium">钉钉</h4>
                <label className="relative inline-flex cursor-pointer items-center">
                  <input
                    type="checkbox"
                    checked={settings.webhook.dingtalk.enabled}
                    onChange={(e) =>
                      updateWebhook("dingtalk", "enabled", e.target.checked)
                    }
                    className="peer sr-only"
                  />
                  <div className="peer h-6 w-11 rounded-full bg-secondary after:absolute after:top-[2px] after:left-[2px] after:h-5 after:w-5 after:rounded-full after:border after:border-muted-foreground/20 after:bg-background after:transition-all after:content-[''] peer-checked:bg-primary peer-checked:after:translate-x-full" />
                </label>
              </div>
              {settings.webhook.dingtalk.enabled && (
                <input
                  type="password"
                  placeholder="加密密钥"
                  value={settings.webhook.dingtalk.secret}
                  onChange={(e) =>
                    updateWebhook("dingtalk", "secret", e.target.value)
                  }
                  className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
                />
              )}
            </div>
          </div>
        </div>

        {/* Security Configuration */}
        <div className="rounded-lg border border-border bg-card p-6">
          <h3 className="text-lg font-semibold mb-4">安全配置</h3>
          <div className="space-y-6">
            {/* Rate Limiting */}
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <h4 className="font-medium">速率限制</h4>
                <label className="relative inline-flex cursor-pointer items-center">
                  <input
                    type="checkbox"
                    checked={settings.security.rate_limit.enabled}
                    onChange={(e) =>
                      updateSecurity("rate_limit", "enabled", e.target.checked)
                    }
                    className="peer sr-only"
                  />
                  <div className="peer h-6 w-11 rounded-full bg-secondary after:absolute after:top-[2px] after:left-[2px] after:h-5 after:w-5 after:rounded-full after:border after:border-muted-foreground/20 after:bg-background after:transition-all after:content-[''] peer-checked:bg-primary peer-checked:after:translate-x-full" />
                </label>
              </div>
              {settings.security.rate_limit.enabled && (
                <div className="grid gap-3 sm:grid-cols-3">
                  <div>
                    <label className="text-sm text-muted-foreground mb-1 block">
                      用户限制
                    </label>
                    <input
                      type="number"
                      min="1"
                      value={settings.security.rate_limit.user_limit}
                      onChange={(e) =>
                        updateSecurity(
                          "rate_limit",
                          "user_limit",
                          parseInt(e.target.value)
                        )
                      }
                      className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
                    />
                  </div>
                  <div>
                    <label className="text-sm text-muted-foreground mb-1 block">
                      IP 限制
                    </label>
                    <input
                      type="number"
                      min="1"
                      value={settings.security.rate_limit.ip_limit}
                      onChange={(e) =>
                        updateSecurity(
                          "rate_limit",
                          "ip_limit",
                          parseInt(e.target.value)
                        )
                      }
                      className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
                    />
                  </div>
                  <div>
                    <label className="text-sm text-muted-foreground mb-1 block">
                      时间窗口 (分钟)
                    </label>
                    <input
                      type="number"
                      min="1"
                      value={settings.security.rate_limit.window_minutes}
                      onChange={(e) =>
                        updateSecurity(
                          "rate_limit",
                          "window_minutes",
                          parseInt(e.target.value)
                        )
                      }
                      className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
                    />
                  </div>
                </div>
              )}
            </div>

            {/* Log Sanitization */}
            <div className="flex items-center justify-between">
              <div>
                <h4 className="font-medium">日志脱敏</h4>
                <p className="text-sm text-muted-foreground">
                  自动隐藏日志中的敏感信息
                </p>
              </div>
              <label className="relative inline-flex cursor-pointer items-center">
                <input
                  type="checkbox"
                  checked={settings.security.log_sanitization.enabled}
                  onChange={(e) =>
                    updateSecurity(
                      "log_sanitization",
                      "enabled",
                      e.target.checked
                    )
                  }
                  className="peer sr-only"
                />
                <div className="peer h-6 w-11 rounded-full bg-secondary after:absolute after:top-[2px] after:left-[2px] after:h-5 after:w-5 after:rounded-full after:border after:border-muted-foreground/20 after:bg-background after:transition-all after:content-[''] peer-checked:bg-primary peer-checked:after:translate-x-full" />
              </label>
            </div>
          </div>
        </div>
      </div>

      {/* Storage Information */}
      <div className="rounded-lg border border-border bg-card p-6">
        <h3 className="text-lg font-semibold mb-4">存储信息</h3>
        <div className="grid gap-4 sm:grid-cols-2">
          <div>
            <label className="text-sm text-muted-foreground mb-1 block">
              数据目录
            </label>
            <div className="rounded-md border border-input bg-muted px-3 py-2 text-sm text-muted-foreground">
              {settings.storage.data_dir || "自动检测"}
            </div>
          </div>
          <div>
            <label className="text-sm text-muted-foreground mb-1 block">
              技能目录
            </label>
            <div className="rounded-md border border-input bg-muted px-3 py-2 text-sm text-muted-foreground">
              {settings.storage.skills_dir || "自动检测"}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
