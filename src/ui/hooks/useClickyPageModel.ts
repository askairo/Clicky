import { useEffect, useMemo, useState } from "react";
import type {
  EditableVar,
  EnvSummary,
  GroupSummary,
  ImportConflictStrategy,
  ImportSummaryDto,
  ImportTargetMode,
  ThemeMode,
} from "../../domain";
import {
  applyEnvFlow,
  appendDraftVar,
  buildImportReq,
  chooseExportPath,
  chooseImportPath,
  createEnvFlow,
  createGroupFlow,
  deleteEnvFlow,
  deleteGroupFlow,
  exportFlow,
  importFlow,
  loadActiveEnvironments,
  loadDraftVariables,
  loadEnvironments,
  loadGroups,
  previewImportFlow,
  renameEnvFlow,
  renameGroupFlow,
  saveVarsFlow,
  type ApplyResultDto,
} from "../../appservice/clickyAppService";
import { isBrowserPreviewRuntimeError } from "../../utils/clickyHelpers";

export function useClickyPageModel() {
  const [theme, setTheme] = useState<ThemeMode>("system");
  const [groups, setGroups] = useState<GroupSummary[]>([]);
  const [selectedGroup, setSelectedGroup] = useState("");
  const [envs, setEnvs] = useState<EnvSummary[]>([]);
  const [selectedEnv, setSelectedEnv] = useState("");
  const [activeEnvs, setActiveEnvs] = useState<string[]>([]);
  const [status, setStatus] = useState("");
  const [busy, setBusy] = useState(false);
  const [applyResult, setApplyResult] = useState<ApplyResultDto | null>(null);
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
  const [importPreview, setImportPreview] = useState<ImportSummaryDto | null>(null);

  const refreshGroups = async (preferred?: string) => {
    const { list, target } = await loadGroups(preferred, selectedGroup);
    setGroups(list);
    setSelectedGroup(target);
  };

  const refreshEnvs = async (groupName: string, preferred?: string) => {
    const { list, target } = await loadEnvironments(groupName, preferred, selectedEnv);
    setEnvs(list);
    setSelectedEnv(target);
  };

  const refreshActiveEnvs = async (groupName: string) => {
    setActiveEnvs(await loadActiveEnvironments(groupName));
  };

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
  }, [theme]);

  useEffect(() => {
    refreshGroups().catch((e) => {
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
    loadDraftVariables(selectedGroup, selectedEnv)
      .then((next) => {
        setDraftVars(next);
        setApplyResult(null);
      })
      .catch((e) => {
        if (!isBrowserPreviewRuntimeError(e)) setStatus(`读取环境失败：${e}`);
      });
  }, [selectedGroup, selectedEnv]);

  const selectedGroupMeta = useMemo(() => groups.find((g) => g.name === selectedGroup), [groups, selectedGroup]);
  const selectedEnvMeta = useMemo(() => envs.find((e) => e.name === selectedEnv), [envs, selectedEnv]);
  const activeLabel = activeEnvs.length === 0 ? "无激活环境" : activeEnvs.join(", ");
  const hasDuplicateKeys = useMemo(() => {
    const keys = draftVars.map((v) => v.key.trim()).filter(Boolean);
    return new Set(keys).size !== keys.length;
  }, [draftVars]);
  const lastApplyOk = applyResult?.variable_results.every((item) => item.applied) ?? false;
  const appliedCount = applyResult?.variable_results.filter((item) => item.applied).length ?? 0;

  const onCreateGroup = async (rawName?: string) => {
    try {
      const result = await createGroupFlow(rawName ?? "", groups);
      if (!result.ok) return setStatus(result.message);
      await refreshGroups(result.name);
      setStatus(result.message);
    } catch (e) {
      setStatus(`创建分组失败：${e}`);
    }
  };

  const onCreateEnv = async (rawName?: string) => {
    try {
      const result = await createEnvFlow(rawName ?? "", selectedGroup, envs);
      if (!result.ok) return setStatus(result.message);
      await refreshEnvs(selectedGroup, result.name);
      setStatus(result.message);
    } catch (e) {
      setStatus(`创建环境失败：${e}`);
    }
  };

  const onAddRow = (key?: string, value?: string) => {
    const result = appendDraftVar(draftVars, key, value);
    if (!result.ok || !result.next) return setStatus(result.message);
    setDraftVars(result.next);
  };

  const onDeleteRow = (idx: number) => {
    const target = draftVars[idx];
    if (!target) return;
    if (!window.confirm(`将删除变量 '${target.key || "(empty)"}'，是否继续？`)) return;
    setDraftVars((prev) => prev.filter((_, i) => i !== idx));
  };

  const onEditRow = (idx: number, field: "key" | "value", value: string) => {
    setDraftVars((prev) => prev.map((row, i) => (i === idx ? { ...row, [field]: value } : row)));
  };

  const onSaveVars = async () => {
    try {
      const result = await saveVarsFlow(selectedGroup, selectedEnv, draftVars);
      if (!result.ok) return setStatus(result.message);
      await refreshEnvs(selectedGroup, selectedEnv);
      setStatus(result.message);
    } catch (e) {
      setStatus(`保存失败：${e}`);
    }
  };

  const onApply = async () => {
    if (!selectedGroup || !selectedEnv) return;
    setBusy(true);
    setStatus("");
    try {
      const result = await applyEnvFlow(selectedGroup, selectedEnv);
      setApplyResult(result.result);
      await refreshActiveEnvs(selectedGroup);
      setStatus(result.message);
    } catch (e) {
      setStatus(`应用失败：${e}`);
      setApplyResult(null);
    } finally {
      setBusy(false);
    }
  };

  const onRenameGroupModal = async () => {
    if (!selectedGroup) return;
    const next = window.prompt("请输入新的分组名", selectedGroup)?.trim() ?? "";
    try {
      const result = await renameGroupFlow(selectedGroup, next, groups);
      if (!result.ok) return setStatus(result.message);
      await refreshGroups(result.next);
      setStatus(result.message);
    } catch (e) {
      setStatus(`重命名分组失败：${e}`);
    }
  };

  const onDeleteGroupModal = async () => {
    if (!selectedGroup) return;
    if (!window.confirm(`将删除分组 '${selectedGroup}'，并删除其下所有环境和变量。是否继续？`)) return;
    const typed = window.prompt(`请输入分组名 '${selectedGroup}' 进行二次确认`);
    if (typed?.trim() !== selectedGroup) return setStatus("二次确认未通过，已取消删除。");
    try {
      const result = await deleteGroupFlow(selectedGroup);
      if (!result.ok) return;
      await refreshGroups();
      setStatus(result.message);
    } catch (e) {
      setStatus(`删除分组失败：${e}`);
    }
  };

  const onRenameEnvModal = async () => {
    if (!selectedGroup || !selectedEnv) return;
    const next = window.prompt("请输入新的环境名", selectedEnv)?.trim() ?? "";
    try {
      const result = await renameEnvFlow(selectedGroup, selectedEnv, next, envs);
      if (!result.ok) return setStatus(result.message);
      await refreshEnvs(selectedGroup, result.next);
      setStatus(result.message);
    } catch (e) {
      setStatus(`重命名环境失败：${e}`);
    }
  };

  const onDeleteEnvModal = async () => {
    if (!selectedGroup || !selectedEnv) return;
    if (!window.confirm(`将删除环境 '${selectedEnv}'，并删除该环境下所有变量。是否继续？`)) return;
    const typed = window.prompt(`请输入环境名 '${selectedEnv}' 进行二次确认`);
    if (typed?.trim() !== selectedEnv) return setStatus("二次确认未通过，已取消删除。");
    try {
      const result = await deleteEnvFlow(selectedGroup, selectedEnv);
      if (!result.ok) return;
      await refreshEnvs(selectedGroup);
      setStatus(result.message);
    } catch (e) {
      setStatus(`删除环境失败：${e}`);
    }
  };

  const onCreateGroupModal = async () => onCreateGroup(window.prompt("请输入分组名称") ?? "");
  const onCreateEnvModal = async () => onCreateEnv(window.prompt("请输入环境名称") ?? "");
  const onAddRowModal = () => {
    const key = window.prompt("请输入变量名（例如 MYSQL_HOST）") ?? "";
    if (!key.trim()) return;
    const value = window.prompt(`请输入 ${key.trim()} 的变量值`) ?? "";
    onAddRow(key, value);
  };

  const toggleExportGroup = (name: string) => {
    setSelectedExportGroups((prev) => (prev.includes(name) ? prev.filter((g) => g !== name) : [...prev, name]));
  };

  const onExportConfig = async () => {
    const path = await chooseExportPath(exportPath);
    if (!path) return;
    setExportPath(path);
    try {
      const result = await exportFlow(path, exportScope, selectedExportGroups);
      if (!result.ok) return setStatus(result.message);
      setStatus(result.message);
    } catch (e) {
      setStatus(`导出失败：${e}`);
    }
  };

  const onPickImportPath = async () => {
    const path = await chooseImportPath();
    if (path) setImportPath(path);
  };

  const onPreviewImport = async () => {
    if (!importPath.trim()) return setStatus("请先选择导入文件。");
    if (importTargetMode === "into_group" && !importTargetGroup.trim()) return setStatus("请选择或输入目标分组。");
    try {
      const req = buildImportReq(importPath, importTargetMode, importTargetGroup, importConflictStrategy);
      const result = await previewImportFlow(req);
      setImportPreview(result.summary);
      setStatus(result.message);
    } catch (e) {
      setStatus(`导入预览失败：${e}`);
    }
  };

  const onImportConfig = async () => {
    if (!importPath.trim()) return setStatus("请先选择导入文件。");
    if (importTargetMode === "into_group" && !importTargetGroup.trim()) return setStatus("请选择或输入目标分组。");
    try {
      const req = buildImportReq(importPath, importTargetMode, importTargetGroup, importConflictStrategy);
      const result = await importFlow(req);
      setImportPreview(result.summary);
      await refreshGroups(importTargetMode === "into_group" ? importTargetGroup.trim() : undefined);
      setStatus(result.message);
    } catch (e) {
      setStatus(`导入失败：${e}`);
    }
  };

  return {
    theme,
    setTheme,
    groups,
    selectedGroup,
    setSelectedGroup,
    envs,
    selectedEnv,
    setSelectedEnv,
    activeEnvs,
    status,
    busy,
    applyResult,
    revealSensitive,
    setRevealSensitive,
    draftVars,
    ioModalOpen,
    setIoModalOpen,
    ioTab,
    setIoTab,
    exportScope,
    setExportScope,
    selectedExportGroups,
    exportPath,
    importPath,
    importTargetMode,
    setImportTargetMode,
    importTargetGroup,
    setImportTargetGroup,
    importConflictStrategy,
    setImportConflictStrategy,
    importPreview,
    selectedGroupMeta,
    selectedEnvMeta,
    activeLabel,
    hasDuplicateKeys,
    lastApplyOk,
    appliedCount,
    onCreateGroupModal,
    onRenameGroupModal,
    onDeleteGroupModal,
    onCreateEnvModal,
    onRenameEnvModal,
    onDeleteEnvModal,
    onAddRowModal,
    onDeleteRow,
    onEditRow,
    onSaveVars,
    onApply,
    toggleExportGroup,
    onExportConfig,
    onPickImportPath,
    onPreviewImport,
    onImportConfig,
  };
}






