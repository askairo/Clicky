import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import envflowLogo from "./assets/envflow-logo.png";
import "./App.css";

type EnvSummary = {
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
  environment: string;
  mode: "persistent";
  variable_results: VariableApplyResult[];
  hook_results: HookResult[];
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
  const [envs, setEnvs] = useState<EnvSummary[]>([]);
  const [selected, setSelected] = useState<string>("");
  const [variables, setVariables] = useState<Record<string, string>>({});
  const [activeEnvs, setActiveEnvs] = useState<string[]>([]);
  const [status, setStatus] = useState<string>("");
  const [busy, setBusy] = useState(false);
  const [applyResult, setApplyResult] = useState<ApplyResult | null>(null);
  const [revealSensitive, setRevealSensitive] = useState(false);

  const refreshActiveEnvs = async () => {
    const names = await invoke<string[]>("detect_active_environments");
    setActiveEnvs(names);
  };

  useEffect(() => {
    (async () => {
      const list = await invoke<EnvSummary[]>("list_environments");
      setEnvs(list);
      if (list.length > 0) {
        setSelected(list[0].name);
      }
      await refreshActiveEnvs();
    })().catch((e) => setStatus(`加载失败：${e}`));
  }, []);

  useEffect(() => {
    if (!selected) return;
    (async () => {
      const vars = await invoke<Record<string, string>>("get_environment_variables", {
        envName: selected,
      });
      setVariables(vars);
      setApplyResult(null);
    })().catch((e) => setStatus(`读取环境失败：${e}`));
  }, [selected]);

  const entries = useMemo(() => Object.entries(variables).sort((a, b) => a[0].localeCompare(b[0])), [variables]);

  const onApply = async () => {
    if (!selected) return;
    setBusy(true);
    setStatus("");
    try {
      const result = await invoke<ApplyResult>("apply_environment", { envName: selected, mode: "persistent" });
      setApplyResult(result);
      await refreshActiveEnvs();
      const okCount = result.variable_results.filter((x) => x.applied).length;
      setStatus(`已应用 ${result.environment}，成功 ${okCount}/${result.variable_results.length} 个变量。请重新打开终端、IDE 或目标应用以读取最新环境变量。`);
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
          <h1>envflow</h1>
          <p className="subtitle">一键切换用户级环境变量（Windows MVP，新进程生效）</p>
        </div>
      </header>

      <section className="panel indicator">
        <strong>当前激活环境：</strong>
        <span>
          {activeEnvs.length === 0 ? "无" : activeEnvs.join(", ")}
        </span>
      </section>

      <section className="panel">
        <label htmlFor="envSelect">环境</label>
        <select id="envSelect" value={selected} onChange={(e) => setSelected(e.target.value)}>
          {envs.length === 0 && <option value="">暂无可用环境</option>}
          {envs.map((e) => (
            <option key={e.name} value={e.name}>
              {e.name} ({e.var_count})
            </option>
          ))}
        </select>

        <p className="notice">应用后会写入 Windows 用户级环境变量；已经打开的终端、IDE 和业务进程通常需要重启后才会读取新值。</p>

        <button onClick={onApply} disabled={!selected || busy}>
          {busy ? "应用中..." : "应用环境"}
        </button>
      </section>

      <section className="panel">
        <div className="section-title">
          <h2>变量预览</h2>
          <label className="toggle">
            <input
              type="checkbox"
              checked={revealSensitive}
              onChange={(e) => setRevealSensitive(e.target.checked)}
            />
            显示敏感值
          </label>
        </div>
        <table>
          <thead>
            <tr>
              <th>变量名</th>
              <th>变量值</th>
            </tr>
          </thead>
          <tbody>
            {entries.map(([k, v]) => (
              <tr key={k}>
                <td>{k}</td>
                <td>{displayValue(k, v, revealSensitive)}</td>
              </tr>
            ))}
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
