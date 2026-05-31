import { open, save } from "@tauri-apps/plugin-dialog";
import {
  applyEnvironment,
  createEnvironment,
  createGroup,
  deleteEnvironment,
  deleteGroup,
  detectActiveEnvironments,
  exportConfig,
  getCurrentEnvSelection,
  getEnvironmentVariables,
  importConfig,
  listEnvironments,
  listGroups,
  previewImportConfig,
  renameEnvironment,
  renameGroup,
  saveEnvironmentVariables,
} from "../service/clickyApi";
import {
  hasDuplicateVarKeys,
  normalizeExportPath,
  toEditableVars,
  toImportRequest,
  toVariablesMap,
} from "../domain/assembler/clickyAssembler";
import type {
  ApplyResultDto,
  EditableVar,
  EnvSummary,
  GroupSummary,
  ImportConflictStrategy,
  ImportSummaryDto,
  ImportTargetMode,
} from "../domain";

export async function loadGroups(preferred: string | undefined, selectedGroup: string) {
  const list = await listGroups();
  const target = preferred || selectedGroup || list[0]?.name || "";
  return { list, target };
}

export async function loadEnvironments(groupName: string, preferred: string | undefined, selectedEnv: string) {
  if (!groupName) return { list: [] as EnvSummary[], target: "" };
  const list = await listEnvironments(groupName);
  const target = preferred || selectedEnv || list[0]?.name || "";
  return { list, target };
}

export async function loadActiveEnvironments(groupName: string) {
  if (!groupName) return [] as string[];
  return detectActiveEnvironments(groupName);
}

export async function loadDraftVariables(groupName: string, envName: string) {
  if (!groupName || !envName) return [] as EditableVar[];
  const vars = await getEnvironmentVariables(groupName, envName);
  return toEditableVars(vars);
}

export async function createGroupFlow(rawName: string, groups: GroupSummary[]): Promise<{ ok: false; message: string } | { ok: true; name: string; message: string }> {
  const name = rawName.trim();
  if (!name) return { ok: false, message: "请输入分组名称。" };
  if (groups.some((g) => g.name === name)) return { ok: false, message: `分组 ${name} 已存在。` };
  await createGroup(name);
  return { ok: true, name, message: `已创建分组：${name}` };
}

export async function createEnvFlow(rawName: string, selectedGroup: string, envs: EnvSummary[]): Promise<{ ok: false; message: string } | { ok: true; name: string; message: string }> {
  const name = rawName.trim();
  if (!selectedGroup) return { ok: false, message: "请先选择分组。" };
  if (!name) return { ok: false, message: "请输入环境名称。" };
  if (envs.some((e) => e.name === name)) return { ok: false, message: `环境 ${name} 已存在。` };
  await createEnvironment(selectedGroup, name);
  return { ok: true, name, message: `已创建环境：${selectedGroup}/${name}` };
}

export function appendDraftVar(draftVars: EditableVar[], key?: string, value?: string): { ok: false; message: string } | { ok: true; next: EditableVar[]; message: string } {
  const nextKey = (key ?? "").trim();
  if (!nextKey) return { ok: false, message: "" };
  if (draftVars.some((item) => item.key.trim() === nextKey)) {
    return { ok: false, message: `变量 ${nextKey} 已存在。` };
  }
  return { ok: true, next: [...draftVars, { key: nextKey, value: value ?? "" }], message: "" };
}

export async function saveVarsFlow(selectedGroup: string, selectedEnv: string, draftVars: EditableVar[]): Promise<{ ok: false; message: string } | { ok: true; variables: Record<string, string>; message: string }> {
  if (!selectedGroup || !selectedEnv) return { ok: false, message: "请选择分组与环境。" };
  if (hasDuplicateVarKeys(draftVars)) return { ok: false, message: "变量名存在重复，请先修正后再保存。" };
  const variables = toVariablesMap(draftVars);
  await saveEnvironmentVariables(selectedGroup, selectedEnv, variables);
  return {
    ok: true,
    variables,
    message: `已保存 ${selectedGroup}/${selectedEnv}，共 ${Object.keys(variables).length} 项。`,
  };
}

export async function applyEnvFlow(selectedGroup: string, selectedEnv: string) {
  const result = await applyEnvironment(selectedGroup, selectedEnv);
  const okCount = result.variable_results.filter((x) => x.applied).length;
  return {
    result,
    message: `已应用 ${result.group}/${result.environment}，成功 ${okCount}/${result.variable_results.length} 个变量。`,
  };
}

export async function renameGroupFlow(selectedGroup: string, nextName: string, groups: GroupSummary[]): Promise<{ ok: false; message: string } | { ok: true; next: string; message: string }> {
  const next = nextName.trim();
  if (!selectedGroup || !next || next === selectedGroup) return { ok: false, message: "" };
  if (groups.some((g) => g.name === next)) return { ok: false, message: `分组 ${next} 已存在。` };
  await renameGroup(selectedGroup, next);
  return { ok: true, next, message: `分组已重命名：${selectedGroup} -> ${next}` };
}

export async function deleteGroupFlow(selectedGroup: string) {
  if (!selectedGroup) return { ok: false, message: "" };
  await deleteGroup(selectedGroup);
  return { ok: true, message: `分组已删除：${selectedGroup}` };
}

export async function renameEnvFlow(selectedGroup: string, selectedEnv: string, nextName: string, envs: EnvSummary[]): Promise<{ ok: false; message: string } | { ok: true; next: string; message: string }> {
  const next = nextName.trim();
  if (!selectedGroup || !selectedEnv || !next || next === selectedEnv) return { ok: false, message: "" };
  if (envs.some((e) => e.name === next)) return { ok: false, message: `环境 ${next} 已存在。` };
  await renameEnvironment(selectedGroup, selectedEnv, next);
  return { ok: true, next, message: `环境已重命名：${selectedEnv} -> ${next}` };
}

export async function deleteEnvFlow(selectedGroup: string, selectedEnv: string) {
  if (!selectedGroup || !selectedEnv) return { ok: false, message: "" };
  await deleteEnvironment(selectedGroup, selectedEnv);
  return { ok: true, message: `环境已删除：${selectedEnv}` };
}

export async function chooseExportPath(exportPath: string) {
  const chosen = await save({
    defaultPath: exportPath.trim() || "clicky-export.yaml",
    filters: [{ name: "YAML", extensions: ["yaml", "yml"] }],
  });
  if (!chosen) return null;
  return normalizeExportPath(String(chosen));
}

export async function exportFlow(path: string, exportScope: "all" | "selected", selectedExportGroups: string[]): Promise<{ ok: false; message: string } | { ok: true; message: string }> {
  const groupNames = exportScope === "all" ? [] : selectedExportGroups;
  if (exportScope === "selected" && groupNames.length === 0) {
    return { ok: false, message: "请至少选择一个导出分组。" };
  }
  const result = await exportConfig(path, groupNames);
  return {
    ok: true,
    message: `导出完成：${result.groups} 组 / ${result.environments} 环境 / ${result.variables} 变量`,
  };
}

export async function chooseImportPath() {
  const chosen = await open({
    multiple: false,
    directory: false,
    filters: [{ name: "YAML", extensions: ["yaml", "yml"] }],
  });
  return chosen ? String(chosen) : null;
}

export function buildImportReq(
  importPath: string,
  importTargetMode: ImportTargetMode,
  importTargetGroup: string,
  importConflictStrategy: ImportConflictStrategy,
) {
  return toImportRequest({
    importPath,
    importTargetMode,
    importTargetGroup,
    importConflictStrategy,
  });
}

export async function previewImportFlow(req: ReturnType<typeof buildImportReq>) {
  const summary = await previewImportConfig(req);
  return { summary, message: "导入预览已生成。" };
}

export async function importFlow(req: ReturnType<typeof buildImportReq>) {
  const summary = await importConfig(req);
  return { summary, message: "导入完成。" };
}

export async function loadCurrentEnvSelection() {
  return getCurrentEnvSelection();
}

export async function syncFromCurrentSelection(
  onGroup: (group: string) => void,
  onEnv: (env: string) => void,
  refresh: {
    groups: (preferred?: string) => Promise<void>;
    envs: (groupName: string, preferred?: string) => Promise<void>;
    activeEnvs: (groupName: string) => Promise<void>;
  },
) {
  const current = await loadCurrentEnvSelection();
  if (!current) return { ok: false as const };
  onGroup(current.group);
  onEnv(current.env);
  await refresh.groups(current.group);
  await refresh.envs(current.group, current.env);
  await refresh.activeEnvs(current.group);
  return { ok: true as const, current };
}

export type { ApplyResultDto, ImportSummaryDto };


