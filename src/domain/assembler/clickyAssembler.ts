import type { EditableVar } from "../vo/clickyVo";
import type { ImportConflictStrategy, ImportRequestDto, ImportTargetMode } from "../dto/clickyDto";

export function toEditableVars(vars: Record<string, string>): EditableVar[] {
  return Object.entries(vars)
    .sort((a, b) => a[0].localeCompare(b[0]))
    .map(([key, value]) => ({ key, value }));
}

export function hasDuplicateVarKeys(draftVars: EditableVar[]): boolean {
  const keys = draftVars.map((v) => v.key.trim()).filter(Boolean);
  return new Set(keys).size !== keys.length;
}

export function toVariablesMap(draftVars: EditableVar[]): Record<string, string> {
  const variables: Record<string, string> = {};
  for (const row of draftVars) {
    const key = row.key.trim();
    if (key) variables[key] = row.value;
  }
  return variables;
}

export function toImportRequest(params: {
  importPath: string;
  importTargetMode: ImportTargetMode;
  importTargetGroup: string;
  importConflictStrategy: ImportConflictStrategy;
}): ImportRequestDto {
  return {
    input_path: params.importPath.trim(),
    target_mode: params.importTargetMode,
    target_group: params.importTargetMode === "into_group" ? params.importTargetGroup.trim() : null,
    conflict_strategy: params.importConflictStrategy,
    dry_run: false,
  };
}

export function normalizeExportPath(pathLike: string): string {
  const path = pathLike.trim();
  return /\.(ya?ml)$/i.test(path) ? path : `${path}.yaml`;
}
