import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import { getFrontendConfig } from "../config/appConfig";
import type { AppConfig } from "../types/config";
import type {
  CreateFileRequest,
  DeleteResult,
  FileContent,
  FsEntry,
  ListWorkspaceRequest,
  RenameEntryRequest,
  WorkspacePathRequest,
  WriteFileRequest,
} from "../types/fs";
import type { StoreStatus } from "../types/store";
import {
  WORKSPACE_CHANGED_EVENT,
  type WorkspaceChangeEvent,
  type WorkspaceWatchStatus,
} from "../types/sync";

type CommandName =
  | "create_workspace_directory"
  | "create_workspace_file"
  | "delete_workspace_entry"
  | "get_app_config"
  | "get_store_status"
  | "get_workspace_watcher_status"
  | "list_workspace_entries"
  | "ping"
  | "read_workspace_file"
  | "rename_workspace_entry"
  | "start_workspace_watcher"
  | "stop_workspace_watcher"
  | "write_workspace_file";

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

export function listWorkspaceEntries(
  request: ListWorkspaceRequest,
): Promise<FsEntry[]> {
  return invokeCommand<FsEntry[]>("list_workspace_entries", { request });
}

export function readWorkspaceFile(
  request: WorkspacePathRequest,
): Promise<FileContent> {
  return invokeCommand<FileContent>("read_workspace_file", { request });
}

export function writeWorkspaceFile(request: WriteFileRequest): Promise<FsEntry> {
  return invokeCommand<FsEntry>("write_workspace_file", { request });
}

export function createWorkspaceFile(request: CreateFileRequest): Promise<FsEntry> {
  return invokeCommand<FsEntry>("create_workspace_file", { request });
}

export function createWorkspaceDirectory(
  request: WorkspacePathRequest,
): Promise<FsEntry> {
  return invokeCommand<FsEntry>("create_workspace_directory", { request });
}

export function renameWorkspaceEntry(
  request: RenameEntryRequest,
): Promise<FsEntry> {
  return invokeCommand<FsEntry>("rename_workspace_entry", { request });
}

export function deleteWorkspaceEntry(
  request: WorkspacePathRequest,
): Promise<DeleteResult> {
  return invokeCommand<DeleteResult>("delete_workspace_entry", { request });
}

export function startWorkspaceWatcher(
  workspaceRoot: string,
): Promise<WorkspaceWatchStatus> {
  return invokeCommand<WorkspaceWatchStatus>("start_workspace_watcher", {
    request: { workspaceRoot },
  });
}

export function stopWorkspaceWatcher(): Promise<WorkspaceWatchStatus> {
  return invokeCommand<WorkspaceWatchStatus>("stop_workspace_watcher");
}

export function getWorkspaceWatcherStatus(): Promise<WorkspaceWatchStatus> {
  return invokeCommand<WorkspaceWatchStatus>("get_workspace_watcher_status");
}

export async function onWorkspaceChanged(
  handler: (event: WorkspaceChangeEvent) => void,
): Promise<UnlistenFn> {
  try {
    return await listen<WorkspaceChangeEvent>(WORKSPACE_CHANGED_EVENT, (event) => {
      handler(event.payload);
    });
  } catch {
    return () => undefined;
  }
}
