import { invoke } from "@tauri-apps/api/core";
import type {
  ApplyResultDto,
  EnvSummary,
  ExportResultDto,
  GroupSummary,
  ImportRequestDto,
  ImportSummaryDto,
  RuntimeCapabilitiesDto,
} from "../domain";

export type EnvSelectionDto = { group: string; env: string };

// Thin IPC wrapper layer: every function here mirrors one Tauri command.
export function listGroups() {
  return invoke<GroupSummary[]>("list_groups");
}

export function listEnvironments(groupName: string) {
  return invoke<EnvSummary[]>("list_environments", { groupName });
}

export function detectActiveEnvironments(groupName: string) {
  return invoke<string[]>("detect_active_environments", { groupName });
}

export function getEnvironmentVariables(groupName: string, envName: string) {
  return invoke<Record<string, string>>("get_environment_variables", { groupName, envName });
}

export function createGroup(groupName: string) {
  return invoke("create_group", { groupName });
}

export function renameGroup(oldName: string, newName: string) {
  return invoke("rename_group", { oldName, newName });
}

export function deleteGroup(groupName: string) {
  return invoke("delete_group", { groupName });
}

export function createEnvironment(groupName: string, envName: string) {
  return invoke("save_environment_variables", { groupName, envName, variables: {} });
}

export function renameEnvironment(groupName: string, oldName: string, newName: string) {
  return invoke("rename_environment", { groupName, oldName, newName });
}

export function deleteEnvironment(groupName: string, envName: string) {
  return invoke("delete_environment", { groupName, envName });
}

export function saveEnvironmentVariables(groupName: string, envName: string, variables: Record<string, string>) {
  return invoke("save_environment_variables", { groupName, envName, variables });
}

export function applyEnvironment(groupName: string, envName: string) {
  // The app only supports persistent mode, so the frontend never passes a mode flag here.
  return invoke<ApplyResultDto>("apply_environment", { groupName, envName, mode: "persistent" });
}

export function exportConfig(outputPath: string, groupNames: string[]) {
  return invoke<ExportResultDto>("export_config", { req: { output_path: outputPath, group_names: groupNames } });
}

export function previewImportConfig(req: ImportRequestDto) {
  return invoke<ImportSummaryDto>("preview_import_config", { req });
}

export function importConfig(req: ImportRequestDto) {
  return invoke<ImportSummaryDto>("import_config", { req });
}

export function getCurrentEnvSelection() {
  return invoke<EnvSelectionDto | null>("get_current_env_selection");
}

export function getRuntimeCapabilities() {
  return invoke<RuntimeCapabilitiesDto>("get_runtime_capabilities");
}
