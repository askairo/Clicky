import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import envflowLogo from "./assets/envflow-logo.png";
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
  if (value === undefined) return "（未设置）";
  if (!isSensitiveKey(key) || reveal) return value;
  return value.length === 0 ? "（空值）" : "••••••••";
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
    })().catch((e) => setStatus(`加载失败：${e}`));
  }, []);

  useEffect(() => {
    if (!selectedGroup) return;
    (async () => {
      await refreshEnvs(selectedGroup);
      await refreshActiveEnvs(selectedGroup);
    })().catch((e) => setStatus(`读取分组失败：${e}`));
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
    })().catch((e) => setStatus(`读取环境失败：${e}`));
  }, [selectedGroup, selectedEnv]);

  const hasDuplicateKeys = useMemo(() => {
    const keys = draftVars.map((v) => v.key.trim()).filter(Boolean);
    return new Set(keys).size !== keys.length;
  }, [draftVars]);

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
      setStatus(`已保存环境 ${selectedGroup}/${selectedEnv}（${Object.keys(variables).length} 项）。`);
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
      setStatus(`已应用 ${result.group}/${result.environment}，成功 ${okCount}/${result.variable_results.length} 个变量。请重新打开终端、IDE 或目标应用以读取最新环境变量。`);
    } catch (e) {
      setStatus(`应用失败：${e}`);
      setApplyResult(null);
    } finally {
      setBusy(false);
    }
  };

  return (
    <main className="container">
      <header className="app-header">
        <img className="app-logo" src={envflowLogo} alt="" aria-hidden="true" />
        <div>
          <p className="subtitle">一键切换用户级环境变量（Windows MVP，新进程生效）</p>
        </div>
        <div className="theme-switch">
          <label htmlFor="themeSelect">主题</label>
          <select id="themeSelect" value={theme} onChange={(e) => setTheme(e.target.value as ThemeMode)}>
            <option value="system">跟随系统</option>
            <option value="light">浅色</option>
            <option value="dark">深色</option>
          </select>
        </div>
      </header>

      <section className="panel indicator">
        <strong>当前激活环境：</strong>
        <span>
          {activeEnvs.length === 0 ? "无" : activeEnvs.join(", ")}
        </span>
      </section>

      <section className="panel">
        <div className="row">
          <div>
            <label htmlFor="groupSelect">分组</label>
            <select id="groupSelect" value={selectedGroup} onChange={(e) => setSelectedGroup(e.target.value)}>
              {groups.length === 0 && <option value="">暂无分组</option>}
              {groups.map((g) => (
                <option key={g.name} value={g.name}>{g.name} ({g.env_count})</option>
              ))}
            </select>
          </div>
          <div>
            <label htmlFor="newGroupName">新建分组</label>
            <div className="inline">
              <input id="newGroupName" value={newGroupName} onChange={(e) => setNewGroupName(e.target.value)} placeholder="例如：mysql" />
              <button onClick={onCreateGroup}>创建</button>
            </div>
          </div>
        </div>

        <div className="row" style={{ marginTop: 10 }}>
          <div>
            <label htmlFor="envSelect">环境</label>
            <select id="envSelect" value={selectedEnv} onChange={(e) => setSelectedEnv(e.target.value)}>
              {envs.length === 0 && <option value="">暂无可用环境</option>}
              {envs.map((e) => (
                <option key={e.name} value={e.name}>{e.name} ({e.var_count})</option>
              ))}
            </select>
          </div>

          <div>
            <label htmlFor="newEnvName">新建环境</label>
            <div className="inline">
              <input id="newEnvName" value={newEnvName} onChange={(e) => setNewEnvName(e.target.value)} placeholder="例如：uat" />
              <button onClick={onCreateEnv}>创建</button>
            </div>
          </div>
        </div>

        <p className="notice">应用后会写入 Windows 用户级环境变量；已经打开的终端、IDE 和业务进程通常需要重启后才会读取新值。</p>

        <button onClick={onApply} disabled={!selectedGroup || !selectedEnv || busy}>
          {busy ? "应用中..." : "应用环境"}
        </button>
      </section>

      <section className="panel">
        <div className="section-title">
          <h2>变量配置（组/环境）</h2>
          <div className="inline">
            <label className="toggle">
              <input
                type="checkbox"
                checked={revealSensitive}
                onChange={(e) => setRevealSensitive(e.target.checked)}
              />
              显示敏感值
            </label>
            <button className="ghost" onClick={onAddRow}>新增变量</button>
            <button onClick={onSaveVars} disabled={!selectedGroup || !selectedEnv}>保存配置</button>
          </div>
        </div>

        {hasDuplicateKeys && <p className="warn">检测到重复变量名，请修正后保存。</p>}

        <table>
          <thead>
            <tr>
              <th>变量名</th>
              <th>变量值</th>
              <th>操作</th>
            </tr>
          </thead>
          <tbody>
            {draftVars.map((row, idx) => (
              <tr key={`row-${idx}`}>
                <td><input value={row.key} onChange={(e) => onEditRow(idx, "key", e.target.value)} /></td>
                <td>
                  <input
                    type={revealSensitive || !isSensitiveKey(row.key) ? "text" : "password"}
                    value={row.value}
                    onChange={(e) => onEditRow(idx, "value", e.target.value)}
                  />
                </td>
                <td><button className="danger" onClick={() => onDeleteRow(idx)}>删除</button></td>
              </tr>
            ))}
            {draftVars.length === 0 && (
              <tr>
                <td colSpan={3}>当前环境暂无变量，点击“新增变量”开始配置。</td>
              </tr>
            )}
          </tbody>
        </table>
      </section>

      {applyResult && (
        <section className="panel">
          <h2>应用结果</h2>
          <table>
            <thead>
              <tr>
                <th>变量名</th>
                <th>应用前</th>
                <th>应用后</th>
                <th>状态</th>
                <th>信息</th>
              </tr>
            </thead>
            <tbody>
              {applyResult.variable_results.map((r) => (
                <tr key={r.key}>
                  <td>{r.key}</td>
                  <td>{displayValue(r.key, r.before, revealSensitive)}</td>
                  <td>{displayValue(r.key, r.after, revealSensitive)}</td>
                  <td>{r.applied ? "成功" : "失败"}</td>
                  <td>{r.message}</td>
                </tr>
              ))}
            </tbody>
          </table>

          {applyResult.hook_results.length > 0 && (
            <>
              <h3>钩子执行</h3>
              <table>
                <thead>
                  <tr>
                    <th>命令</th>
                    <th>状态</th>
                    <th>返回码</th>
                    <th>信息</th>
                  </tr>
                </thead>
                <tbody>
                  {applyResult.hook_results.map((h, idx) => (
                    <tr key={`${h.command}-${idx}`}>
                      <td>{h.command}</td>
                      <td>{h.success ? "成功" : "失败"}</td>
                      <td>{h.code ?? "-"}</td>
                      <td>{h.message || "-"}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </>
          )}
        </section>
      )}

      {status && <p className="status">{status}</p>}
    </main>
  );
}

export default App;
