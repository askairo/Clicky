import { listen } from "@tauri-apps/api/event";
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
  loadRuntimeCapabilities,
  loadActiveEnvironments,
  loadDraftVariables,
  loadEnvironments,
  loadGroups,
  previewImportFlow,
  renameEnvFlow,
  renameGroupFlow,
  saveVarsFlow,
  syncFromCurrentSelection,
} from "../../appservice/clickyAppService";
import { isBrowserPreviewRuntimeError } from "../../utils/clickyHelpers";

export function useClickyPageModel() {
  type ActionModalPayload = {
    title: string;
    description?: string;
    confirmLabel: string;
    danger?: boolean;
    primaryLabel?: string;
    primaryPlaceholder?: string;
    primaryValue: string;
    secondaryLabel?: string;
    secondaryPlaceholder?: string;
    secondaryValue: string;
    requireMatchText?: string;
    onConfirm: (args: { primary: string; secondary: string }) => Promise<void> | void;
  };

  const [theme, setTheme] = useState<ThemeMode>("system");
  const [groups, setGroups] = useState<GroupSummary[]>([]);
  const [selectedGroup, setSelectedGroup] = useState("");
  const [envs, setEnvs] = useState<EnvSummary[]>([]);
  const [selectedEnv, setSelectedEnv] = useState("");
  const [activeEnvs, setActiveEnvs] = useState<string[]>([]);
  const [status, setStatus] = useState("");
  const [runtimeHint, setRuntimeHint] = useState("");
  const [busy, setBusy] = useState(false);
  const [revealSensitive, setRevealSensitive] = useState(false);
  const [draftVars, setDraftVars] = useState<EditableVar[]>([]);
  const [baseVars, setBaseVars] = useState<EditableVar[]>([]);
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
  const [actionModal, setActionModal] = useState<ActionModalPayload | null>(null);

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
    loadRuntimeCapabilities()
      .then((caps) => {
        setRuntimeHint(caps.apply_scope_hint);
      })
      .catch((e) => {
        if (!isBrowserPreviewRuntimeError(e)) setStatus(`读取平台能力失败：${e}`);
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
        setBaseVars(next);
      })
      .catch((e) => {
        if (!isBrowserPreviewRuntimeError(e)) setStatus(`读取环境失败：${e}`);
      });
  }, [selectedGroup, selectedEnv]);

  useEffect(() => {
    if (!status) return;
    const timer = window.setTimeout(() => {
      setStatus("");
    }, 4000);
    return () => window.clearTimeout(timer);
  }, [status]);

  useEffect(() => {
    let unlistenStatus: undefined | (() => void);
    let unlistenSwitched: undefined | (() => void);
    listen<string>("tray-switch-status", (event) => {
      setStatus(event.payload);
      syncFromCurrentSelection(setSelectedGroup, setSelectedEnv, {
        groups: refreshGroups,
        envs: refreshEnvs,
        activeEnvs: refreshActiveEnvs,
      }).catch((e) => {
        if (!isBrowserPreviewRuntimeError(e)) setStatus(`同步托盘状态失败：${e}`);
      });
    })
      .then((dispose) => {
        unlistenStatus = dispose;
      })
      .catch(() => {
        // Browser preview has no tauri bridge.
      });

    listen<{ group: string; env: string }>("tray-switched-env", (event) => {
      const { group, env } = event.payload;
      (async () => {
        setSelectedGroup(group);
        setSelectedEnv(env);
        await refreshGroups(group);
        await refreshEnvs(group, env);
        await refreshActiveEnvs(group);
      })().catch((e) => {
        if (!isBrowserPreviewRuntimeError(e)) setStatus(`同步托盘状态失败：${e}`);
      });
    })
      .then((dispose) => {
        unlistenSwitched = dispose;
      })
      .catch(() => {
        // Browser preview has no tauri bridge.
      });
    return () => {
      unlistenStatus?.();
      unlistenSwitched?.();
    };
  }, []);

  const selectedGroupMeta = useMemo(() => groups.find((g) => g.name === selectedGroup), [groups, selectedGroup]);
  const selectedEnvMeta = useMemo(() => envs.find((e) => e.name === selectedEnv), [envs, selectedEnv]);
  const activeLabel = activeEnvs.length === 0 ? "无激活环境" : activeEnvs.join(", ");
  const selectedLabel = selectedGroup && selectedEnv ? `${selectedGroup}/${selectedEnv}` : "未选择环境";
  const appliedLabel =
    selectedGroup && activeEnvs.length > 0 ? activeEnvs.map((env) => `${selectedGroup}/${env}`).join(", ") : "未应用环境";
  const hasDuplicateKeys = useMemo(() => {
    const keys = draftVars.map((v) => v.key.trim()).filter(Boolean);
    return new Set(keys).size !== keys.length;
  }, [draftVars]);
  const hasUnsavedChanges = useMemo(() => {
    const normalize = (rows: EditableVar[]) =>
      rows
        .map((row) => ({ key: row.key.trim(), value: row.value }))
        .filter((row) => row.key.length > 0)
        .sort((a, b) => a.key.localeCompare(b.key));
    return JSON.stringify(normalize(draftVars)) !== JSON.stringify(normalize(baseVars));
  }, [draftVars, baseVars]);

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
    if (key === undefined && value === undefined) {
      setDraftVars((prev) => [...prev, { key: "", value: "" }]);
      return;
    }
    const result = appendDraftVar(draftVars, key, value);
    if (!result.ok || !result.next) return setStatus(result.message);
    setDraftVars(result.next);
  };

  const onDeleteRow = (idx: number) => {
    const target = draftVars[idx];
    if (!target) return;
    setActionModal({
      title: "删除变量",
      description: `将删除变量“${target.key || "(empty)"}”，确认继续吗？`,
      confirmLabel: "删除",
      danger: true,
      primaryValue: "",
      secondaryValue: "",
      onConfirm: () => {
        setDraftVars((prev) => prev.filter((_, i) => i !== idx));
      },
    });
  };

  const onEditRow = (idx: number, field: "key" | "value", value: string) => {
    setDraftVars((prev) => prev.map((row, i) => (i === idx ? { ...row, [field]: value } : row)));
  };

  const onSaveVars = async () => {
    try {
      const result = await saveVarsFlow(selectedGroup, selectedEnv, draftVars);
      if (!result.ok) return setStatus(result.message);
      await refreshEnvs(selectedGroup, selectedEnv);
      const refreshed = await loadDraftVariables(selectedGroup, selectedEnv);
      setDraftVars(refreshed);
      setBaseVars(refreshed);
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
      await refreshActiveEnvs(selectedGroup);
      const { total, failed, changed } = result.result.summary;
      const target = `${selectedGroup}/${selectedEnv}`;
      const baseMessage =
        failed === 0
          ? `已应用 ${target}：处理 ${total} 个，实际变更 ${changed} 个。`
          : `已应用 ${target}：处理 ${total} 个，实际变更 ${changed} 个，失败 ${failed} 个。`;
      const message = runtimeHint ? `${baseMessage}\n${runtimeHint}` : baseMessage;
      setStatus(message);
    } catch (e) {
      setStatus(`应用失败：${e}`);
    } finally {
      setBusy(false);
    }
  };

  const onRenameGroupModal = async () => {
    if (!selectedGroup) return;
    setActionModal({
      title: "重命名分组",
      description: "请输入新的分组名称。",
      confirmLabel: "确认",
      primaryLabel: "分组名称",
      primaryPlaceholder: "请输入分组名称",
      primaryValue: selectedGroup,
      secondaryValue: "",
      onConfirm: async ({ primary }) => {
        try {
          const result = await renameGroupFlow(selectedGroup, primary.trim(), groups);
          if (!result.ok) return setStatus(result.message);
          await refreshGroups(result.next);
          setStatus(result.message);
        } catch (e) {
          setStatus(`重命名分组失败：${e}`);
        }
      },
    });
  };

  const onDeleteGroupModal = async () => {
    if (!selectedGroup) return;
    setActionModal({
      title: "删除分组",
      description: `将删除分组“${selectedGroup}”及其下所有环境和变量。请输入分组名进行确认。`,
      confirmLabel: "确认删除",
      danger: true,
      primaryLabel: "分组名称确认",
      primaryPlaceholder: selectedGroup,
      primaryValue: "",
      secondaryValue: "",
      requireMatchText: selectedGroup,
      onConfirm: async () => {
        try {
          const result = await deleteGroupFlow(selectedGroup);
          if (!result.ok) return;
          await refreshGroups();
          setStatus(result.message);
        } catch (e) {
          setStatus(`删除分组失败：${e}`);
        }
      },
    });
  };

  const onRenameEnvModal = async () => {
    if (!selectedGroup || !selectedEnv) return;
    setActionModal({
      title: "重命名环境",
      description: "请输入新的环境名称。",
      confirmLabel: "确认",
      primaryLabel: "环境名称",
      primaryPlaceholder: "请输入环境名称",
      primaryValue: selectedEnv,
      secondaryValue: "",
      onConfirm: async ({ primary }) => {
        try {
          const result = await renameEnvFlow(selectedGroup, selectedEnv, primary.trim(), envs);
          if (!result.ok) return setStatus(result.message);
          await refreshEnvs(selectedGroup, result.next);
          setStatus(result.message);
        } catch (e) {
          setStatus(`重命名环境失败：${e}`);
        }
      },
    });
  };

  const onDeleteEnvModal = async () => {
    if (!selectedGroup || !selectedEnv) return;
    setActionModal({
      title: "删除环境",
      description: `将删除环境“${selectedEnv}”及其下所有变量。请输入环境名进行确认。`,
      confirmLabel: "确认删除",
      danger: true,
      primaryLabel: "环境名称确认",
      primaryPlaceholder: selectedEnv,
      primaryValue: "",
      secondaryValue: "",
      requireMatchText: selectedEnv,
      onConfirm: async () => {
        try {
          const result = await deleteEnvFlow(selectedGroup, selectedEnv);
          if (!result.ok) return;
          await refreshEnvs(selectedGroup);
          setStatus(result.message);
        } catch (e) {
          setStatus(`删除环境失败：${e}`);
        }
      },
    });
  };

  const onCreateGroupModal = async () => {
    setActionModal({
      title: "新建分组",
      description: "请输入分组名称。",
      confirmLabel: "创建",
      primaryLabel: "分组名称",
      primaryPlaceholder: "例如 znder-erp",
      primaryValue: "",
      secondaryValue: "",
      onConfirm: async ({ primary }) => onCreateGroup(primary),
    });
  };
  const onCreateEnvModal = async () => {
    setActionModal({
      title: "新建环境",
      description: "请输入环境名称。将自动复制同分组下已有变量 Key（值留空）。",
      confirmLabel: "创建",
      primaryLabel: "环境名称",
      primaryPlaceholder: "例如 dev / sit / prod",
      primaryValue: "",
      secondaryValue: "",
      onConfirm: async ({ primary }) => onCreateEnv(primary),
    });
  };
  const onAddRowModal = () => {
    setActionModal({
      title: "新增变量",
      description: "新增后会同步到同分组其他环境（仅同步 Key，其他环境值默认为空）。",
      confirmLabel: "添加",
      primaryLabel: "变量名",
      primaryPlaceholder: "例如 MYSQL_HOST",
      primaryValue: "",
      secondaryLabel: "变量值",
      secondaryPlaceholder: "请输入变量值",
      secondaryValue: "",
      onConfirm: ({ primary, secondary }) => {
        onAddRow(primary, secondary);
      },
    });
  };

  const closeActionModal = () => setActionModal(null);
  const onActionModalPrimaryChange = (value: string) =>
    setActionModal((prev) => (prev ? { ...prev, primaryValue: value } : prev));
  const onActionModalSecondaryChange = (value: string) =>
    setActionModal((prev) => (prev ? { ...prev, secondaryValue: value } : prev));
  const onActionModalConfirm = async () => {
    if (!actionModal) return;
    if (actionModal.requireMatchText && actionModal.primaryValue.trim() !== actionModal.requireMatchText) {
      setStatus("二次确认未通过，已取消操作。");
      return;
    }
    await actionModal.onConfirm({
      primary: actionModal.primaryValue,
      secondary: actionModal.secondaryValue,
    });
    setActionModal(null);
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
    runtimeHint,
    busy,
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
    selectedLabel,
    appliedLabel,
    hasDuplicateKeys,
    hasUnsavedChanges,
    onCreateGroupModal,
    onRenameGroupModal,
    onDeleteGroupModal,
    onCreateEnvModal,
    onRenameEnvModal,
    onDeleteEnvModal,
    onAddRow,
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
    actionModal,
    closeActionModal,
    onActionModalPrimaryChange,
    onActionModalSecondaryChange,
    onActionModalConfirm,
  };
}
