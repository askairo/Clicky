import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
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
  mode: "session" | "persistent";
  variable_results: VariableApplyResult[];
  hook_results: HookResult[];
};

function App() {
  const [envs, setEnvs] = useState<EnvSummary[]>([]);
  const [selected, setSelected] = useState<string>("");
  const [mode, setMode] = useState<"session" | "persistent">("session");
  const [variables, setVariables] = useState<Record<string, string>>({});
  const [activeEnvs, setActiveEnvs] = useState<string[]>([]);
  const [status, setStatus] = useState<string>("");
  const [busy, setBusy] = useState(false);
  const [applyResult, setApplyResult] = useState<ApplyResult | null>(null);

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
      const result = await invoke<ApplyResult>("apply_environment", { envName: selected, mode });
      setApplyResult(result);
      await refreshActiveEnvs();
      const okCount = result.variable_results.filter((x) => x.applied).length;
      setStatus(`已应用 ${result.environment}（${result.mode}），成功 ${okCount}/${result.variable_results.length} 个变量。`);
    } catch (e) {
      setStatus(`应用失败：${e}`);
      setApplyResult(null);
    } finally {
      setBusy(false);
    }
  };

  return (
    <main className="container">
      <h1>envflow</h1>
      <p className="subtitle">一键切换环境变量（Windows MVP）</p>

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

        <label htmlFor="modeSelect">应用模式</label>
        <select id="modeSelect" value={mode} onChange={(e) => setMode(e.target.value as "session" | "persistent")}>
          <option value="session">session（当前进程）</option>
          <option value="persistent">persistent（新进程生效）</option>
        </select>

        <button onClick={onApply} disabled={!selected || busy}>
          {busy ? "应用中..." : "应用环境"}
        </button>
      </section>

      <section className="panel">
        <h2>变量预览</h2>
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
                <td>{v}</td>
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
                  <td>{r.before ?? "（未设置）"}</td>
                  <td>{r.after}</td>
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
