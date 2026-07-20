import { invoke } from "@tauri-apps/api/core";

import { getFrontendConfig } from "../config/appConfig";
import type { AppConfig } from "../types/config";
import type { StoreStatus } from "../types/store";

type CommandName = "get_app_config" | "get_store_status" | "ping";

export type IpcError = {
  code: string;
  message: string;
};

function isIpcError(value: unknown): value is IpcError {
  if (!value || typeof value !== "object") {
    return false;
  }

  const candidate = value as Partial<IpcError>;
  return typeof candidate.code === "string" && typeof candidate.message === "string";
}

async function invokeCommand<TResponse>(
  command: CommandName,
  args?: Record<string, unknown>,
): Promise<TResponse> {
  try {
    return await invoke<TResponse>(command, args);
  } catch (error) {
    if (isIpcError(error)) {
      throw new Error(`${error.code}: ${error.message}`);
    }

    throw error instanceof Error ? error : new Error(String(error));
  }
}

export async function getAppConfig(): Promise<AppConfig> {
  try {
    return await invokeCommand<AppConfig>("get_app_config");
  } catch {
    return getFrontendConfig();
  }
}

export async function pingCore(): Promise<string> {
  return invokeCommand<string>("ping");
}

export async function getStoreStatus(): Promise<StoreStatus | null> {
  try {
    return await invokeCommand<StoreStatus>("get_store_status");
  } catch {
    return null;
  }
}
