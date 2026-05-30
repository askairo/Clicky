export type GroupSummary = {
  name: string;
  description?: string;
  env_count: number;
};

export type EnvSummary = {
  group: string;
  name: string;
  description?: string;
  var_count: number;
};

export type VariableApplyResult = {
  key: string;
  before?: string;
  after: string;
  applied: boolean;
  message: string;
};

export type HookResult = {
  command: string;
  success: boolean;
  code?: number;
  message: string;
};
