import type { HookResult, VariableApplyResult } from "../entity/clickyEntity";

export type ApplyResultDto = {
  group: string;
  environment: string;
  mode: "persistent";
  variable_results: VariableApplyResult[];
  hook_results: HookResult[];
};

export type ExportResultDto = {
  output_path: string;
  groups: number;
  environments: number;
  variables: number;
};

export type ImportSummaryDto = {
  groups_added: number;
  groups_skipped: number;
  envs_added: number;
  envs_skipped: number;
  vars_added: number;
  vars_overwritten: number;
  vars_skipped: number;
};

export type ImportTargetMode = "keep_groups" | "into_group";
export type ImportConflictStrategy = "skip_existing" | "overwrite_existing" | "only_add_new";

export type ImportRequestDto = {
  input_path: string;
  target_mode: ImportTargetMode;
  target_group: string | null;
  conflict_strategy: ImportConflictStrategy;
  dry_run: boolean;
};
