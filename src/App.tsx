import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  CheckCircle2,
  ChevronRight,
  CircleAlert,
  Eye,
  EyeOff,
  FolderPlus,
  Loader2,
  Moon,
  Plus,
  Save,
  Settings2,
  Sparkles,
  Sun,
  Trash2,
  Wand2,
} from "lucide-react";
import clickyLogo from "./assets/clicky-logo.png";
import "./App.css";

type ThemeMode = "system" | "light" | "dark";

type GroupSummary = {
  name: string;
  description?: string;
  env_count: number;
};

type EnvSummary = {
  group: string;
  name: string;
  description?: string;
  var_count: number;
};

type VariableApplyResult = {
  key: string;
  before?: string;
  after: string;
  applied: boolean;
  message: string;
};

type HookResult = {
  command: string;
  success: boolean;
  code?: number;
  message: string;
};

type ApplyResult = {
  group: string;
  environment: string;
  mode: "persistent";
  variable_results: VariableApplyResult[];
  hook_results: HookResult[];
};

type EditableVar = {
  key: string;
  value: string;
};

const SENSITIVE_KEY_PATTERN = /(pass|password|pwd|token|secret|key|credential|auth)/i;

function isSensitiveKey(key: string) {
  return SENSITIVE_KEY_PATTERN.test(key);
}

function displayValue(key: string, value?: string, reveal = false) {
  if (value === undefined) return "未设置";
  if (!isSensitiveKey(key) || reveal) return value;
  return value.length === 0 ? "空值" : "••••••••";
}

function themeLabel(theme: ThemeMode) {
  if (theme === "light") return "浅色";
  if (theme === "dark") return "深色";
  return "系统";
}

function isBrowserPreviewRuntimeError(error: unknown) {
  return String(error).includes("Cannot read properties of undefined (reading 'invoke')");
}

function App() {
  const [theme, setTheme] = useState<ThemeMode>("system");
  const [groups, setGroups] = useState<GroupSummary[]>([]);
  const [selectedGroup, setSelectedGroup] = useState<string>("");
  const [newGroupName, setNewGroupName] = useState<string>("");

  const [envs, setEnvs] = useState<EnvSummary[]>([]);
  const [selectedEnv, setSelectedEnv] = useState<string>("");
  const [newEnvName, setNewEnvName] = useState<string>("");

  const [activeEnvs, setActiveEnvs] = useState<string[]>([]);
  const [status, setStatus] = useState<string>("");
  const [busy, setBusy] = useState(false);
  const [applyResult, setApplyResult] = useState<ApplyResult | null>(null);
  const [revealSensitive, setRevealSensitive] = useState(false);
  const [draftVars, setDraftVars] = useState<EditableVar[]>([]);

  const refreshGroups = async (preferred?: string) => {
    const list = await invoke<GroupSummary[]>("list_groups");
    setGroups(list);
    const target = preferred || selectedGroup || list[0]?.name || "";
    setSelectedGroup(target);
  };

  const refreshEnvs = async (groupName: string, preferred?: string) => {
    if (!groupName) {
      setEnvs([]);
      setSelectedEnv("");
      return;
    }
    const list = await invoke<EnvSummary[]>("list_environments", { groupName });
    setEnvs(list);
    const target = preferred || selectedEnv || list[0]?.name || "";
    setSelectedEnv(target);
  };

  const refreshActiveEnvs = async (groupName: string) => {
    if (!groupName) {
      setActiveEnvs([]);
      return;
    }
    const names = await invoke<string[]>("detect_active_environments", { groupName });
    setActiveEnvs(names);
  };

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
  }, [theme]);

  useEffect(() => {
    (async () => {
      await refreshGroups();
    })().catch((e) => {
      if (!isBrowserPreviewRuntimeError(e)) setStatus(`加载失败：${e}`);
    });
  }, []);

  useEffect(() => {
    if (!selectedGroup) return;
    (async () => {
      await refreshEnvs(selectedGroup);
      await refreshActiveEnvs(selectedGroup);
    })().catch((e) => {
      if (!isBrowserPreviewRuntimeError(e)) setStatus(`读取分组失败：${e}`);
    });
  }, [selectedGroup]);

  useEffect(() => {
    if (!selectedGroup || !selectedEnv) {
      setDraftVars([]);
      return;
    }
    (async () => {
      const vars = await invoke<Record<string, string>>("get_environment_variables", {
        groupName: selectedGroup,
        envName: selectedEnv,
      });
      const next = Object.entries(vars)
        .sort((a, b) => a[0].localeCompare(b[0]))
        .map(([key, value]) => ({ key, value }));
      setDraftVars(next);
      setApplyResult(null);
    })().catch((e) => {
      if (!isBrowserPreviewRuntimeError(e)) setStatus(`读取环境失败：${e}`);
    });
  }, [selectedGroup, selectedEnv]);

  const selectedGroupMeta = useMemo(
    () => groups.find((group) => group.name === selectedGroup),
    [groups, selectedGroup],
  );

  const selectedEnvMeta = useMemo(
    () => envs.find((env) => env.name === selectedEnv),
    [envs, selectedEnv],
  );

  const activeLabel = activeEnvs.length === 0 ? "无激活环境" : activeEnvs.join(", ");

  const hasDuplicateKeys = useMemo(() => {
    const keys = draftVars.map((v) => v.key.trim()).filter(Boolean);
    return new Set(keys).size !== keys.length;
  }, [draftVars]);

  const lastApplyOk = applyResult?.variable_results.every((item) => item.applied) ?? false;
  const appliedCount = applyResult?.variable_results.filter((item) => item.applied).length ?? 0;

  const onCreateGroup = async () => {
    const name = newGroupName.trim();
    if (!name) {
      setStatus("请输入分组名称。");
      return;
    }
    if (groups.some((g) => g.name === name)) {
      setStatus(`分组 ${name} 已存在。`);
      return;
    }

    try {
      await invoke("create_group", { groupName: name });
      await refreshGroups(name);
      setNewGroupName("");
      setStatus(`已创建分组：${name}`);
    } catch (e) {
      setStatus(`创建分组失败：${e}`);
    }
  };

  const onCreateEnv = async () => {
    const name = newEnvName.trim();
    if (!selectedGroup) {
      setStatus("请先选择分组。");
      return;
    }
    if (!name) {
      setStatus("请输入环境名称。");
      return;
    }
    if (envs.some((e) => e.name === name)) {
      setStatus(`环境 ${name} 已存在。`);
      return;
    }

    try {
      await invoke("save_environment_variables", {
        groupName: selectedGroup,
        envName: name,
        variables: {},
      });
      await refreshEnvs(selectedGroup, name);
      setNewEnvName("");
      setStatus(`已创建环境：${selectedGroup}/${name}`);
    } catch (e) {
      setStatus(`创建环境失败：${e}`);
    }
  };

  const onAddRow = () => {
    setDraftVars((prev) => [...prev, { key: "", value: "" }]);
  };

  const onDeleteRow = (idx: number) => {
    setDraftVars((prev) => prev.filter((_, i) => i !== idx));
  };

  const onEditRow = (idx: number, field: "key" | "value", value: string) => {
    setDraftVars((prev) => prev.map((row, i) => (i === idx ? { ...row, [field]: value } : row)));
  };

  const onSaveVars = async () => {
    if (!selectedGroup || !selectedEnv) return;
    if (hasDuplicateKeys) {
      setStatus("变量名存在重复，请先修正后再保存。");
      return;
    }

    const variables: Record<string, string> = {};
    for (const row of draftVars) {
      const key = row.key.trim();
      if (!key) continue;
      variables[key] = row.value;
    }

    try {
      await invoke("save_environment_variables", {
        groupName: selectedGroup,
        envName: selectedEnv,
        variables,
      });
      await refreshEnvs(selectedGroup, selectedEnv);
      setStatus(`已保存 ${selectedGroup}/${selectedEnv}，共 ${Object.keys(variables).length} 项。`);
    } catch (e) {
      setStatus(`保存失败：${e}`);
    }
  };

  const onApply = async () => {
    if (!selectedGroup || !selectedEnv) return;
    setBusy(true);
    setStatus("");
    try {
      const result = await invoke<ApplyResult>("apply_environment", {
        groupName: selectedGroup,
        envName: selectedEnv,
        mode: "persistent",
      });
      setApplyResult(result);
      await refreshActiveEnvs(selectedGroup);
      const okCount = result.variable_results.filter((x) => x.applied).length;
      setStatus(`已应用 ${result.group}/${result.environment}，成功 ${okCount}/${result.variable_results.length} 个变量。`);
    } catch (e) {
      setStatus(`应用失败：${e}`);
      setApplyResult(null);
    } finally {
      setBusy(false);
    }
  };

  return (
    <main className="app-shell">
      <header className="topbar">
        <div className="brand">
          <img className="app-logo" src={clickyLogo} alt="" aria-hidden="true" />
          <div>
            <h1>Clicky</h1>
            <p>Environment switcher</p>
          </div>
        </div>

        <div className="topbar-status" aria-label="当前状态">
          <span className="state-dot" />
          <span>{selectedGroup && selectedEnv ? `${selectedGroup} / ${selectedEnv}` : "未选择环境"}</span>
        </div>

        <div className="theme-control" role="group" aria-label="主题">
          {(["system", "light", "dark"] as ThemeMode[]).map((mode) => (
            <button
              key={mode}
              className={theme === mode ? "icon-button active" : "icon-button"}
              onClick={() => setTheme(mode)}
              title={themeLabel(mode)}
              aria-label={themeLabel(mode)}
              type="button"
            >
              {mode === "system" && <Sparkles size={16} />}
              {mode === "light" && <Sun size={16} />}
              {mode === "dark" && <Moon size={16} />}
            </button>
          ))}
        </div>
      </header>

      <section className="summary-band" aria-label="环境概览">
        <div>
          <span className="eyebrow">Active</span>
          <strong>{activeLabel}</strong>
        </div>
        <div>
          <span className="eyebrow">Groups</span>
          <strong>{groups.length}</strong>
        </div>
        <div>
          <span className="eyebrow">Variables</span>
          <strong>{draftVars.filter((row) => row.key.trim()).length}</strong>
        </div>
        <button className="primary-action" onClick={onApply} disabled={!selectedGroup || !selectedEnv || busy}>
          {busy ? <Loader2 className="spin" size={17} /> : <Wand2 size={17} />}
          {busy ? "应用中" : "应用环境"}
        </button>
      </section>

      <div className="workspace">
        <aside className="sidebar" aria-label="环境导航">
          <section className="sidebar-section">
            <div className="section-heading">
              <span>分组</span>
              <span>{groups.length}</span>
            </div>
            <div className="item-list">
              {groups.map((group) => (
                <button
                  key={group.name}
                  className={group.name === selectedGroup ? "nav-item selected" : "nav-item"}
                  onClick={() => setSelectedGroup(group.name)}
                  type="button"
                >
                  <span>
                    <strong>{group.name}</strong>
                    <small>{group.env_count} 个环境</small>
                  </span>
                  <ChevronRight size={15} />
                </button>
              ))}
              {groups.length === 0 && <div className="empty-note">暂无分组</div>}
            </div>
            <div className="create-line">
              <input
                value={newGroupName}
                onChange={(e) => setNewGroupName(e.target.value)}
                placeholder="新分组"
              />
              <button className="icon-button solid" onClick={onCreateGroup} title="创建分组" aria-label="创建分组">
                <FolderPlus size={16} />
              </button>
            </div>
          </section>

          <section className="sidebar-section">
            <div className="section-heading">
              <span>环境</span>
              <span>{envs.length}</span>
            </div>
            <div className="item-list">
              {envs.map((env) => {
                const isActive = activeEnvs.includes(env.name);
                return (
                  <button
                    key={env.name}
                    className={env.name === selectedEnv ? "nav-item selected" : "nav-item"}
                    onClick={() => setSelectedEnv(env.name)}
                    type="button"
                  >
                    <span>
                      <strong>{env.name}</strong>
                      <small>{env.var_count} 个变量{isActive ? " · 已激活" : ""}</small>
                    </span>
                    {isActive ? <CheckCircle2 size={15} /> : <ChevronRight size={15} />}
                  </button>
                );
              })}
              {envs.length === 0 && <div className="empty-note">暂无环境</div>}
            </div>
            <div className="create-line">
              <input
                value={newEnvName}
                onChange={(e) => setNewEnvName(e.target.value)}
                placeholder="新环境"
              />
              <button className="icon-button solid" onClick={onCreateEnv} title="创建环境" aria-label="创建环境">
                <Plus size={16} />
              </button>
            </div>
          </section>
        </aside>

        <section className="workbench">
          <div className="workbench-header">
            <div>
              <span className="eyebrow">Workspace</span>
              <h2>{selectedEnv || "选择一个环境"}</h2>
              <p>{selectedGroupMeta?.description || selectedEnvMeta?.description || "Windows 用户级环境变量"}</p>
            </div>
            <div className="toolbar">
              <button
                className="ghost-action"
                onClick={() => setRevealSensitive((value) => !value)}
                type="button"
              >
                {revealSensitive ? <EyeOff size={16} /> : <Eye size={16} />}
                {revealSensitive ? "隐藏敏感值" : "显示敏感值"}
              </button>
              <button className="ghost-action" onClick={onAddRow} type="button">
                <Plus size={16} />
                新增变量
              </button>
              <button className="save-action" onClick={onSaveVars} disabled={!selectedGroup || !selectedEnv}>
                <Save size={16} />
                保存
              </button>
            </div>
          </div>

          {hasDuplicateKeys && (
            <div className="inline-alert">
              <CircleAlert size={16} />
              <span>变量名重复，请修正后保存。</span>
            </div>
          )}

          <div className="table-shell">
            <table>
              <thead>
                <tr>
                  <th>变量名</th>
                  <th>变量值</th>
                  <th aria-label="操作" />
                </tr>
              </thead>
              <tbody>
                {draftVars.map((row, idx) => (
                  <tr key={`row-${idx}`}>
                    <td>
                      <input
                        className="cell-input mono"
                        value={row.key}
                        onChange={(e) => onEditRow(idx, "key", e.target.value)}
                        placeholder="VARIABLE_NAME"
                      />
                    </td>
                    <td>
                      <input
                        className="cell-input"
                        type={revealSensitive || !isSensitiveKey(row.key) ? "text" : "password"}
                        value={row.value}
                        onChange={(e) => onEditRow(idx, "value", e.target.value)}
                        placeholder="value"
                      />
                    </td>
                    <td className="row-actions">
                      <button
                        className="icon-button danger"
                        onClick={() => onDeleteRow(idx)}
                        title="删除变量"
                        aria-label="删除变量"
                        type="button"
                      >
                        <Trash2 size={15} />
                      </button>
                    </td>
                  </tr>
                ))}
                {draftVars.length === 0 && (
                  <tr>
                    <td className="empty-table" colSpan={3}>
                      <Settings2 size={18} />
                      <span>当前环境暂无变量</span>
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>

          {applyResult && (
            <section className={lastApplyOk ? "result-panel success" : "result-panel warning"}>
              <div className="result-heading">
                <div>
                  <span className="eyebrow">Apply Result</span>
                  <h3>
                    {appliedCount}/{applyResult.variable_results.length} 已应用
                  </h3>
                </div>
                {lastApplyOk ? <CheckCircle2 size={20} /> : <CircleAlert size={20} />}
              </div>

              <div className="table-shell compact">
                <table>
                  <thead>
                    <tr>
                      <th>变量名</th>
                      <th>应用前</th>
                      <th>应用后</th>
                      <th>状态</th>
                    </tr>
                  </thead>
                  <tbody>
                    {applyResult.variable_results.map((result) => (
                      <tr key={result.key}>
                        <td className="mono">{result.key}</td>
                        <td>{displayValue(result.key, result.before, revealSensitive)}</td>
                        <td>{displayValue(result.key, result.after, revealSensitive)}</td>
                        <td>{result.applied ? "成功" : result.message}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>

              {applyResult.hook_results.length > 0 && (
                <div className="hook-list">
                  {applyResult.hook_results.map((hook, idx) => (
                    <div className="hook-item" key={`${hook.command}-${idx}`}>
                      <span className="mono">{hook.command}</span>
                      <strong>{hook.success ? "成功" : "失败"}</strong>
                    </div>
                  ))}
                </div>
              )}
            </section>
          )}
        </section>
      </div>

      {status && (
        <div className="toast" role="status">
          {status}
        </div>
      )}
    </main>
  );
}

export default App;
