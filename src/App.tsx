import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  CheckCircle2,
  ChevronRight,
  CircleAlert,
  Edit3,
  Eye,
  EyeOff,
  Loader2,
  Moon,
  Plus,
  Save,
  Settings2,
  Sparkles,
  Sun,
  Trash2,
  Wand2,
  X,
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

type ExportResult = {
  output_path: string;
  groups: number;
  environments: number;
  variables: number;
};

type ImportSummary = {
  groups_added: number;
  groups_skipped: number;
  envs_added: number;
  envs_skipped: number;
  vars_added: number;
  vars_overwritten: number;
  vars_skipped: number;
};

type ImportTargetMode = "keep_groups" | "into_group";
type ImportConflictStrategy = "skip_existing" | "overwrite_existing" | "only_add_new";

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

  const [envs, setEnvs] = useState<EnvSummary[]>([]);
  const [selectedEnv, setSelectedEnv] = useState<string>("");

  const [activeEnvs, setActiveEnvs] = useState<string[]>([]);
  const [status, setStatus] = useState<string>("");
  const [busy, setBusy] = useState(false);
  const [applyResult, setApplyResult] = useState<ApplyResult | null>(null);
  const [revealSensitive, setRevealSensitive] = useState(false);
  const [draftVars, setDraftVars] = useState<EditableVar[]>([]);
  const [ioModalOpen, setIoModalOpen] = useState(false);
  const [ioTab, setIoTab] = useState<"export" | "import">("export");
  const [exportScope, setExportScope] = useState<"all" | "selected">("all");
  const [selectedExportGroups, setSelectedExportGroups] = useState<string[]>([]);
  const [exportPath, setExportPath] = useState("");
  const [importPath, setImportPath] = useState("");
  const [importTargetMode, setImportTargetMode] = useState<ImportTargetMode>("keep_groups");
  const [importTargetGroup, setImportTargetGroup] = useState("");
  const [importConflictStrategy, setImportConflictStrategy] =
    useState<ImportConflictStrategy>("skip_existing");
  const [importPreview, setImportPreview] = useState<ImportSummary | null>(null);

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

  const onCreateGroup = async (rawName?: string) => {
    const name = (rawName ?? "").trim();
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
      setStatus(`已创建分组：${name}`);
    } catch (e) {
      setStatus(`创建分组失败：${e}`);
    }
  };

  const onCreateEnv = async (rawName?: string) => {
    const name = (rawName ?? "").trim();
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
      setStatus(`已创建环境：${selectedGroup}/${name}`);
    } catch (e) {
      setStatus(`创建环境失败：${e}`);
    }
  };

  const onAddRow = (key?: string, value?: string) => {
    const nextKey = (key ?? "").trim();
    if (!nextKey) return;
    if (draftVars.some((item) => item.key.trim() === nextKey)) {
      setStatus(`变量 ${nextKey} 已存在。`);
      return;
    }
    setDraftVars((prev) => [...prev, { key: nextKey, value: value ?? "" }]);
  };

  const onDeleteRow = (idx: number) => {
    const target = draftVars[idx];
    if (!target) return;
    const ok = window.confirm(`将删除变量 '${target.key || "(empty)"}'，是否继续？`);
    if (!ok) return;
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

  const onRenameGroupModal = async () => {
    if (!selectedGroup) return;
    const next = window.prompt("请输入新的分组名", selectedGroup)?.trim();
    if (!next || next === selectedGroup) return;
    if (groups.some((g) => g.name === next)) {
      setStatus(`分组 ${next} 已存在。`);
      return;
    }
    try {
      await invoke("rename_group", { oldName: selectedGroup, newName: next });
      await refreshGroups(next);
      setStatus(`分组已重命名：${selectedGroup} -> ${next}`);
    } catch (e) {
      setStatus(`重命名分组失败：${e}`);
    }
  };

  const onDeleteGroupModal = async () => {
    if (!selectedGroup) return;
    const ok = window.confirm(`将删除分组 '${selectedGroup}'，并删除其下所有环境和变量。是否继续？`);
    if (!ok) return;
    const typed = window.prompt(`请输入分组名 '${selectedGroup}' 进行二次确认`);
    if (typed?.trim() !== selectedGroup) {
      setStatus("二次确认未通过，已取消删除。");
      return;
    }
    try {
      await invoke("delete_group", { groupName: selectedGroup });
      await refreshGroups();
      setStatus(`分组已删除：${selectedGroup}`);
    } catch (e) {
      setStatus(`删除分组失败：${e}`);
    }
  };

  const onRenameEnvModal = async () => {
    if (!selectedGroup || !selectedEnv) return;
    const next = window.prompt("请输入新的环境名", selectedEnv)?.trim();
    if (!next || next === selectedEnv) return;
    if (envs.some((e) => e.name === next)) {
      setStatus(`环境 ${next} 已存在。`);
      return;
    }
    try {
      await invoke("rename_environment", { groupName: selectedGroup, oldName: selectedEnv, newName: next });
      await refreshEnvs(selectedGroup, next);
      setStatus(`环境已重命名：${selectedEnv} -> ${next}`);
    } catch (e) {
      setStatus(`重命名环境失败：${e}`);
    }
  };

  const onDeleteEnvModal = async () => {
    if (!selectedGroup || !selectedEnv) return;
    const ok = window.confirm(`将删除环境 '${selectedEnv}'，并删除该环境下所有变量。是否继续？`);
    if (!ok) return;
    const typed = window.prompt(`请输入环境名 '${selectedEnv}' 进行二次确认`);
    if (typed?.trim() !== selectedEnv) {
      setStatus("二次确认未通过，已取消删除。");
      return;
    }
    try {
      await invoke("delete_environment", { groupName: selectedGroup, envName: selectedEnv });
      await refreshEnvs(selectedGroup);
      setStatus(`环境已删除：${selectedEnv}`);
    } catch (e) {
      setStatus(`删除环境失败：${e}`);
    }
  };

  const onCreateGroupModal = async () => {
    const name = window.prompt("请输入分组名称") ?? "";
    await onCreateGroup(name);
  };

  const onCreateEnvModal = async () => {
    const name = window.prompt("请输入环境名称") ?? "";
    await onCreateEnv(name);
  };

  const onAddRowModal = () => {
    const key = window.prompt("请输入变量名（例如 MYSQL_HOST）") ?? "";
    if (!key.trim()) return;
    const value = window.prompt(`请输入 ${key.trim()} 的变量值`) ?? "";
    onAddRow(key, value);
  };

  const toggleExportGroup = (name: string) => {
    setSelectedExportGroups((prev) =>
      prev.includes(name) ? prev.filter((g) => g !== name) : [...prev, name],
    );
  };

  const onExportConfig = async () => {
    const chosen = await save({
      defaultPath: exportPath.trim() || "clicky-export.yaml",
      filters: [{ name: "YAML", extensions: ["yaml", "yml"] }],
    });
    if (!chosen) return;
    let path = String(chosen).trim();
    if (!/\.(ya?ml)$/i.test(path)) path = `${path}.yaml`;
    setExportPath(path);
    const groupNames = exportScope === "all" ? [] : selectedExportGroups;
    if (exportScope === "selected" && groupNames.length === 0) {
      setStatus("请至少选择一个导出分组。");
      return;
    }
    try {
      const result = await invoke<ExportResult>("export_config", {
        req: { output_path: path, group_names: groupNames },
      });
      setStatus(`导出完成：${result.groups} 组 / ${result.environments} 环境 / ${result.variables} 变量`);
    } catch (e) {
      setStatus(`导出失败：${e}`);
    }
  };

  const onPickImportPath = async () => {
    const chosen = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "YAML", extensions: ["yaml", "yml"] }],
    });
    if (!chosen) return;
    setImportPath(String(chosen));
  };

  const buildImportReq = () => ({
    input_path: importPath.trim(),
    target_mode: importTargetMode,
    target_group: importTargetMode === "into_group" ? importTargetGroup.trim() : null,
    conflict_strategy: importConflictStrategy,
    dry_run: false,
  });

  const onPreviewImport = async () => {
    if (!importPath.trim()) {
      setStatus("请先选择导入文件。");
      return;
    }
    if (importTargetMode === "into_group" && !importTargetGroup.trim()) {
      setStatus("请选择或输入目标分组。");
      return;
    }
    try {
      const summary = await invoke<ImportSummary>("preview_import_config", { req: buildImportReq() });
      setImportPreview(summary);
      setStatus("导入预览已生成。");
    } catch (e) {
      setStatus(`导入预览失败：${e}`);
    }
  };

  const onImportConfig = async () => {
    if (!importPath.trim()) {
      setStatus("请先选择导入文件。");
      return;
    }
    if (importTargetMode === "into_group" && !importTargetGroup.trim()) {
      setStatus("请选择或输入目标分组。");
      return;
    }
    try {
      const summary = await invoke<ImportSummary>("import_config", { req: buildImportReq() });
      setImportPreview(summary);
      await refreshGroups(importTargetMode === "into_group" ? importTargetGroup.trim() : undefined);
      setStatus("导入完成。");
    } catch (e) {
      setStatus(`导入失败：${e}`);
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
              <div className="section-heading-actions">
                <button className="icon-button" type="button" title="新建分组" onClick={onCreateGroupModal}>
                  <Plus size={14} />
                </button>
                <button className="icon-button" type="button" title="重命名分组" onClick={onRenameGroupModal}>
                  <Edit3 size={14} />
                </button>
                <button className="icon-button danger" type="button" title="删除分组" onClick={onDeleteGroupModal}>
                  <Trash2 size={14} />
                </button>
              </div>
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
          </section>

          <section className="sidebar-section">
            <div className="section-heading">
              <span>环境</span>
              <div className="section-heading-actions">
                <button className="icon-button" type="button" title="新建环境" onClick={onCreateEnvModal}>
                  <Plus size={14} />
                </button>
                <button className="icon-button" type="button" title="重命名环境" onClick={onRenameEnvModal}>
                  <Edit3 size={14} />
                </button>
                <button className="icon-button danger" type="button" title="删除环境" onClick={onDeleteEnvModal}>
                  <Trash2 size={14} />
                </button>
              </div>
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
                onClick={() => {
                  setIoModalOpen(true);
                  setIoTab("export");
                }}
                type="button"
              >
                <Settings2 size={16} />
                导入/导出
              </button>
              <button
                className="ghost-action"
                onClick={() => setRevealSensitive((value) => !value)}
                type="button"
              >
                {revealSensitive ? <EyeOff size={16} /> : <Eye size={16} />}
                {revealSensitive ? "隐藏敏感值" : "显示敏感值"}
              </button>
              <button className="ghost-action" onClick={onAddRowModal} type="button">
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

      {ioModalOpen && (
        <div className="modal-backdrop" role="presentation" onClick={() => setIoModalOpen(false)}>
          <section className="io-modal" role="dialog" aria-modal="true" onClick={(e) => e.stopPropagation()}>
            <header className="io-modal-header">
              <div>
                <span className="eyebrow">Config I/O</span>
                <h3>导入 / 导出</h3>
              </div>
              <button className="icon-button" type="button" onClick={() => setIoModalOpen(false)} aria-label="关闭">
                <X size={16} />
              </button>
            </header>

            <div className="segment-control" role="tablist" aria-label="导入导出标签">
              <button className={ioTab === "export" ? "segment active" : "segment"} type="button" onClick={() => setIoTab("export")}>
                导出
              </button>
              <button className={ioTab === "import" ? "segment active" : "segment"} type="button" onClick={() => setIoTab("import")}>
                导入
              </button>
            </div>

            {ioTab === "export" && (
              <div className="io-modal-body">
                <div className="segment-control">
                  <button className={exportScope === "all" ? "segment active" : "segment"} type="button" onClick={() => setExportScope("all")}>
                    全量导出
                  </button>
                  <button className={exportScope === "selected" ? "segment active" : "segment"} type="button" onClick={() => setExportScope("selected")}>
                    按分组导出
                  </button>
                </div>

                {exportScope === "selected" && (
                  <div className="tag-grid">
                    {groups.map((g) => (
                      <button
                        key={`export-${g.name}`}
                        type="button"
                        className={selectedExportGroups.includes(g.name) ? "tag-chip active" : "tag-chip"}
                        onClick={() => toggleExportGroup(g.name)}
                      >
                        {g.name}
                      </button>
                    ))}
                  </div>
                )}

                <div className="io-path-line">
                  <input value={exportPath} readOnly placeholder="导出路径将在保存后显示" />
                  <button className="save-action" type="button" onClick={onExportConfig}>
                    导出到文件
                  </button>
                </div>
              </div>
            )}

            {ioTab === "import" && (
              <div className="io-modal-body">
                <div className="io-path-line">
                  <input value={importPath} readOnly placeholder="请选择导入文件（.yaml/.yml）" />
                  <button className="ghost-action" type="button" onClick={onPickImportPath}>
                    选择文件
                  </button>
                </div>

                <div className="io-form-grid">
                  <label>
                    目标模式
                    <select value={importTargetMode} onChange={(e) => setImportTargetMode(e.target.value as ImportTargetMode)}>
                      <option value="keep_groups">保持原分组</option>
                      <option value="into_group">导入到指定分组</option>
                    </select>
                  </label>
                  <label>
                    冲突策略
                    <select value={importConflictStrategy} onChange={(e) => setImportConflictStrategy(e.target.value as ImportConflictStrategy)}>
                      <option value="skip_existing">跳过已存在（默认）</option>
                      <option value="overwrite_existing">覆盖已存在</option>
                      <option value="only_add_new">仅新增</option>
                    </select>
                  </label>
                </div>

                {importTargetMode === "into_group" && (
                  <div className="io-path-line">
                    <input value={importTargetGroup} onChange={(e) => setImportTargetGroup(e.target.value)} placeholder="目标分组名" />
                  </div>
                )}

                <div className="io-actions">
                  <button className="ghost-action" type="button" onClick={onPreviewImport}>
                    预览导入
                  </button>
                  <button className="save-action" type="button" onClick={onImportConfig}>
                    确认导入
                  </button>
                </div>

                {importPreview && (
                  <div className="preview-stats">
                    <div>
                      <span className="eyebrow">分组 新增/跳过</span>
                      <strong>{importPreview.groups_added}/{importPreview.groups_skipped}</strong>
                    </div>
                    <div>
                      <span className="eyebrow">环境 新增/跳过</span>
                      <strong>{importPreview.envs_added}/{importPreview.envs_skipped}</strong>
                    </div>
                    <div>
                      <span className="eyebrow">变量 新增/覆盖/跳过</span>
                      <strong>{importPreview.vars_added}/{importPreview.vars_overwritten}/{importPreview.vars_skipped}</strong>
                    </div>
                  </div>
                )}
              </div>
            )}
          </section>
        </div>
      )}

      {status && (
        <div className="toast" role="status">
          {status}
        </div>
      )}
    </main>
  );
}

export default App;
