import { useEffect, useState } from "react";

interface SkillInfo {
  id: string;
  name: string;
  version: string;
  description: string;
  author: string;
  tags: string[];
  enabled: boolean;
}

export function Skills() {
  const [skills, setSkills] = useState<SkillInfo[]>([]);
  void setSkills;
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [searchTerm, setSearchTerm] = useState("");

  useEffect(() => {
    async function fetchSkills() {
      try {
        // This will be connected to Tauri command
        // const data = await invoke<SkillInfo[]>("list_skills");
        // setSkills(data);
        setError("Tauri backend not yet connected");
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to fetch skills");
      } finally {
        setLoading(false);
      }
    }

    fetchSkills();
  }, []);

  const filteredSkills = skills.filter(skill =>
    skill.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
    skill.description.toLowerCase().includes(searchTerm.toLowerCase()) ||
    skill.tags.some(tag => tag.toLowerCase().includes(searchTerm.toLowerCase()))
  );

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-3xl font-bold tracking-tight">技能管理</h2>
          <p className="text-muted-foreground">
            管理和执行 Desktop Agent 技能
          </p>
        </div>
        <button className="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90">
          安装技能
        </button>
      </div>

      <div className="flex gap-4">
        <input
          type="text"
          placeholder="搜索技能..."
          value={searchTerm}
          onChange={(e) => setSearchTerm(e.target.value)}
          className="flex-1 rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
        />
      </div>

      {loading ? (
        <div className="flex items-center justify-center py-12">
          <div className="text-muted-foreground">加载中...</div>
        </div>
      ) : error ? (
        <div className="rounded-lg border border-destructive bg-destructive/10 p-4 text-destructive">
          {error}
        </div>
      ) : filteredSkills.length === 0 ? (
        <div className="rounded-lg border border-border bg-card p-12 text-center">
          <div className="text-4xl mb-4">🔧</div>
          <h3 className="text-lg font-semibold mb-2">暂无技能</h3>
          <p className="text-muted-foreground mb-4">
            {searchTerm ? "没有找到匹配的技能" : "开始安装第一个技能吧"}
          </p>
          <button className="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90">
            浏览技能市场
          </button>
        </div>
      ) : (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {filteredSkills.map((skill) => (
            <div
              key={skill.id}
              className="rounded-lg border border-border bg-card p-6 transition-colors hover:bg-accent/50"
            >
              <div className="flex items-start justify-between mb-4">
                <div className="flex-1">
                  <h3 className="font-semibold text-lg">{skill.name}</h3>
                  <p className="text-sm text-muted-foreground">v{skill.version}</p>
                </div>
                <button
                  className={`rounded-md px-2 py-1 text-xs font-medium ${
                    skill.enabled
                      ? "bg-green-500/10 text-green-600"
                      : "bg-muted text-muted-foreground"
                  }`}
                  aria-label={skill.enabled ? "禁用" : "启用"}
                >
                  {skill.enabled ? "已启用" : "已禁用"}
                </button>
              </div>

              <p className="text-sm text-muted-foreground mb-4">
                {skill.description}
              </p>

              <div className="flex flex-wrap gap-2 mb-4">
                {skill.tags.map((tag) => (
                  <span
                    key={tag}
                    className="rounded-full bg-secondary px-2 py-1 text-xs text-secondary-foreground"
                  >
                    {tag}
                  </span>
                ))}
              </div>

              <div className="flex items-center justify-between">
                <span className="text-xs text-muted-foreground">
                  作者: {skill.author}
                </span>
                <button className="text-sm text-primary hover:text-primary/80">
                  执行
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
