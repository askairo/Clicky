import type { ThemeMode } from "../domain";

const SENSITIVE_KEY_PATTERN = /(pass|password|pwd|token|secret|key|credential|auth)/i;

export function isSensitiveKey(key: string) {
  return SENSITIVE_KEY_PATTERN.test(key);
}

export function displayValue(key: string, value?: string | null, reveal = false) {
  if (value == null) return "未设置";
  if (!isSensitiveKey(key) || reveal) return value;
  return value.length === 0 ? "空值" : "••••••••";
}

export function themeLabel(theme: ThemeMode) {
  if (theme === "light") return "浅色";
  if (theme === "dark") return "深色";
  return "系统";
}

export function isBrowserPreviewRuntimeError(error: unknown) {
  return String(error).includes("Cannot read properties of undefined (reading 'invoke')");
}

