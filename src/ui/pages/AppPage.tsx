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
import "../styles/app.css";
import { useClickyPageModel } from "../hooks/useClickyPageModel";
import type { ImportConflictStrategy, ImportTargetMode, ThemeMode } from "../../domain";
import { displayValue, isSensitiveKey, themeLabel } from "../../utils/clickyHelpers";

function App() {
  const c = useClickyPageModel();

  return (
    <main className="app-shell">
      <header className="topbar">
        <div className="brand">
          <img className="app-logo" src="/clicky-logo.png" alt="" aria-hidden="true" />
          <div>
            <h1>Clicky</h1>
            <p>Environment switcher</p>
          </div>
        </div>

        <div className="topbar-status" aria-label="当前状态">
          <span className="state-dot" />
          <span>{c.selectedGroup && c.selectedEnv ? `${c.selectedGroup} / ${c.selectedEnv}` : "未选择环境"}</span>
        </div>

        <div className="theme-control" role="group" aria-label="主题">
          {(["system", "light", "dark"] as ThemeMode[]).map((mode) => (
            <button
              key={mode}
              className={c.theme === mode ? "icon-button active" : "icon-button"}
              onClick={() => c.setTheme(mode)}
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
          <strong>{c.activeLabel}</strong>
        </div>
        <div>
          <span className="eyebrow">Groups</span>
          <strong>{c.groups.length}</strong>
        </div>
        <div>
          <span className="eyebrow">Variables</span>
          <strong>{c.draftVars.filter((row) => row.key.trim()).length}</strong>
        </div>
        <button className="primary-action" onClick={c.onApply} disabled={!c.selectedGroup || !c.selectedEnv || c.busy}>
          {c.busy ? <Loader2 className="spin" size={17} /> : <Wand2 size={17} />}
          {c.busy ? "应用中" : "应用环境"}
        </button>
      </section>

      <div className="workspace">
        <aside className="sidebar" aria-label="环境导航">
          <section className="sidebar-section">
            <div className="section-heading">
              <span>分组</span>
              <div className="section-heading-actions">
                <button className="icon-button" type="button" title="新建分组" onClick={c.onCreateGroupModal}><Plus size={14} /></button>
                <button className="icon-button" type="button" title="重命名分组" onClick={c.onRenameGroupModal}><Edit3 size={14} /></button>
                <button className="icon-button danger" type="button" title="删除分组" onClick={c.onDeleteGroupModal}><Trash2 size={14} /></button>
              </div>
            </div>
            <div className="item-list">
              {c.groups.map((group) => (
                <button
                  key={group.name}
                  className={group.name === c.selectedGroup ? "nav-item selected" : "nav-item"}
                  onClick={() => c.setSelectedGroup(group.name)}
                  type="button"
                >
                  <span>
                    <strong>{group.name}</strong>
                    <small>{group.env_count} 个环境</small>
                  </span>
                  <ChevronRight size={15} />
                </button>
              ))}
              {c.groups.length === 0 && <div className="empty-note">暂无分组</div>}
            </div>
          </section>

          <section className="sidebar-section">
            <div className="section-heading">
              <span>环境</span>
              <div className="section-heading-actions">
                <button className="icon-button" type="button" title="新建环境" onClick={c.onCreateEnvModal}><Plus size={14} /></button>
                <button className="icon-button" type="button" title="重命名环境" onClick={c.onRenameEnvModal}><Edit3 size={14} /></button>
                <button className="icon-button danger" type="button" title="删除环境" onClick={c.onDeleteEnvModal}><Trash2 size={14} /></button>
              </div>
            </div>
            <div className="item-list">
              {c.envs.map((env) => {
                const isActive = c.activeEnvs.includes(env.name);
                return (
                  <button
                    key={env.name}
                    className={env.name === c.selectedEnv ? "nav-item selected" : "nav-item"}
                    onClick={() => c.setSelectedEnv(env.name)}
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
              {c.envs.length === 0 && <div className="empty-note">暂无环境</div>}
            </div>
          </section>
        </aside>

        <section className="workbench">
          <div className="workbench-header">
            <div>
              <span className="eyebrow">Workspace</span>
              <h2>{c.selectedEnv || "选择一个环境"}</h2>
              <p>{c.selectedGroupMeta?.description || c.selectedEnvMeta?.description || "Windows 用户级环境变量"}</p>
            </div>
            <div className="toolbar">
              <button className="ghost-action" onClick={() => { c.setIoModalOpen(true); c.setIoTab("export"); }} type="button">
                <Settings2 size={16} />导入/导出
              </button>
              <button className="ghost-action" onClick={() => c.setRevealSensitive((value) => !value)} type="button">
                {c.revealSensitive ? <EyeOff size={16} /> : <Eye size={16} />}
                {c.revealSensitive ? "隐藏敏感值" : "显示敏感值"}
              </button>
              <button className="ghost-action" onClick={c.onAddRowModal} type="button"><Plus size={16} />新增变量</button>
              <button className="save-action" onClick={c.onSaveVars} disabled={!c.selectedGroup || !c.selectedEnv}><Save size={16} />保存</button>
            </div>
          </div>

          {c.hasDuplicateKeys && (
            <div className="inline-alert"><CircleAlert size={16} /><span>变量名重复，请修正后保存。</span></div>
          )}

          <div className="table-shell">
            <table>
              <thead><tr><th>变量名</th><th>变量值</th><th aria-label="操作" /></tr></thead>
              <tbody>
                {c.draftVars.map((row, idx) => (
                  <tr key={`row-${idx}`}>
                    <td><input className="cell-input mono" value={row.key} onChange={(e) => c.onEditRow(idx, "key", e.target.value)} placeholder="VARIABLE_NAME" /></td>
                    <td><input className="cell-input" type={c.revealSensitive || !isSensitiveKey(row.key) ? "text" : "password"} value={row.value} onChange={(e) => c.onEditRow(idx, "value", e.target.value)} placeholder="value" /></td>
                    <td className="row-actions"><button className="icon-button danger" onClick={() => c.onDeleteRow(idx)} title="删除变量" aria-label="删除变量" type="button"><Trash2 size={15} /></button></td>
                  </tr>
                ))}
                {c.draftVars.length === 0 && (
                  <tr><td className="empty-table" colSpan={3}><Settings2 size={18} /><span>当前环境暂无变量</span></td></tr>
                )}
              </tbody>
            </table>
          </div>

          {c.applyResult && (
            <section className={c.lastApplyOk ? "result-panel success" : "result-panel warning"}>
              <div className="result-heading">
                <div><span className="eyebrow">Apply Result</span><h3>{c.appliedCount}/{c.applyResult.variable_results.length} 已应用</h3></div>
                {c.lastApplyOk ? <CheckCircle2 size={20} /> : <CircleAlert size={20} />}
              </div>
              <div className="table-shell compact">
                <table>
                  <thead><tr><th>变量名</th><th>应用前</th><th>应用后</th><th>状态</th></tr></thead>
                  <tbody>
                    {c.applyResult.variable_results.map((result) => (
                      <tr key={result.key}>
                        <td className="mono">{result.key}</td>
                        <td>{displayValue(result.key, result.before, c.revealSensitive)}</td>
                        <td>{displayValue(result.key, result.after, c.revealSensitive)}</td>
                        <td>{result.applied ? "成功" : result.message}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              {c.applyResult.hook_results.length > 0 && (
                <div className="hook-list">
                  {c.applyResult.hook_results.map((hook, idx) => (
                    <div className="hook-item" key={`${hook.command}-${idx}`}><span className="mono">{hook.command}</span><strong>{hook.success ? "成功" : "失败"}</strong></div>
                  ))}
                </div>
              )}
            </section>
          )}
        </section>
      </div>

      {c.ioModalOpen && (
        <div className="modal-backdrop" role="presentation" onClick={() => c.setIoModalOpen(false)}>
          <section className="io-modal" role="dialog" aria-modal="true" onClick={(e) => e.stopPropagation()}>
            <header className="io-modal-header">
              <div><span className="eyebrow">Config I/O</span><h3>导入 / 导出</h3></div>
              <button className="icon-button" type="button" onClick={() => c.setIoModalOpen(false)} aria-label="关闭"><X size={16} /></button>
            </header>

            <div className="segment-control" role="tablist" aria-label="导入导出标签">
              <button className={c.ioTab === "export" ? "segment active" : "segment"} type="button" onClick={() => c.setIoTab("export")}>导出</button>
              <button className={c.ioTab === "import" ? "segment active" : "segment"} type="button" onClick={() => c.setIoTab("import")}>导入</button>
            </div>

            {c.ioTab === "export" && (
              <div className="io-modal-body">
                <div className="segment-control">
                  <button className={c.exportScope === "all" ? "segment active" : "segment"} type="button" onClick={() => c.setExportScope("all")}>全量导出</button>
                  <button className={c.exportScope === "selected" ? "segment active" : "segment"} type="button" onClick={() => c.setExportScope("selected")}>按分组导出</button>
                </div>

                {c.exportScope === "selected" && (
                  <div className="tag-grid">
                    {c.groups.map((g) => (
                      <button key={`export-${g.name}`} type="button" className={c.selectedExportGroups.includes(g.name) ? "tag-chip active" : "tag-chip"} onClick={() => c.toggleExportGroup(g.name)}>{g.name}</button>
                    ))}
                  </div>
                )}

                <div className="io-path-line">
                  <input value={c.exportPath} readOnly placeholder="导出路径将在保存后显示" />
                  <button className="save-action" type="button" onClick={c.onExportConfig}>导出到文件</button>
                </div>
              </div>
            )}

            {c.ioTab === "import" && (
              <div className="io-modal-body">
                <div className="io-path-line">
                  <input value={c.importPath} readOnly placeholder="请选择导入文件（.yaml/.yml）" />
                  <button className="ghost-action" type="button" onClick={c.onPickImportPath}>选择文件</button>
                </div>

                <div className="io-form-grid">
                  <label>目标模式
                    <select value={c.importTargetMode} onChange={(e) => c.setImportTargetMode(e.target.value as ImportTargetMode)}>
                      <option value="keep_groups">保持原分组</option>
                      <option value="into_group">导入到指定分组</option>
                    </select>
                  </label>
                  <label>冲突策略
                    <select value={c.importConflictStrategy} onChange={(e) => c.setImportConflictStrategy(e.target.value as ImportConflictStrategy)}>
                      <option value="skip_existing">跳过已存在（默认）</option>
                      <option value="overwrite_existing">覆盖已存在</option>
                      <option value="only_add_new">仅新增</option>
                    </select>
                  </label>
                </div>

                {c.importTargetMode === "into_group" && (
                  <div className="io-path-line"><input value={c.importTargetGroup} onChange={(e) => c.setImportTargetGroup(e.target.value)} placeholder="目标分组名" /></div>
                )}

                <div className="io-actions">
                  <button className="ghost-action" type="button" onClick={c.onPreviewImport}>预览导入</button>
                  <button className="save-action" type="button" onClick={c.onImportConfig}>确认导入</button>
                </div>

                {c.importPreview && (
                  <div className="preview-stats">
                    <div><span className="eyebrow">分组 新增/跳过</span><strong>{c.importPreview.groups_added}/{c.importPreview.groups_skipped}</strong></div>
                    <div><span className="eyebrow">环境 新增/跳过</span><strong>{c.importPreview.envs_added}/{c.importPreview.envs_skipped}</strong></div>
                    <div><span className="eyebrow">变量 新增/覆盖/跳过</span><strong>{c.importPreview.vars_added}/{c.importPreview.vars_overwritten}/{c.importPreview.vars_skipped}</strong></div>
                  </div>
                )}
              </div>
            )}
          </section>
        </div>
      )}

      {c.status && <div className="toast" role="status">{c.status}</div>}
    </main>
  );
}

export default App;



