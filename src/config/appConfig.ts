import type { AppConfig } from "../types/config";

function normalizeOptional(value: string | undefined): string | null {
  const trimmedValue = value?.trim();
  return trimmedValue ? trimmedValue : null;
}

export function getFrontendConfig(): AppConfig {
  return {
    defaultWorkspaceRoot: normalizeOptional(
      import.meta.env.VITE_DEFAULT_WORKSPACE_ROOT,
    ),
    latexToolchainPath: normalizeOptional(import.meta.env.VITE_LATEX_TOOLCHAIN_PATH),
  };
}
