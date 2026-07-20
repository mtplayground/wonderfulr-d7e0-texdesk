export const WORKSPACE_CHANGED_EVENT = "workspace-changed";

export type WorkspaceChangeEvent = {
  workspaceRoot: string;
  paths: string[];
  kind: string;
};

export type WorkspaceWatchStatus = {
  active: boolean;
  workspaceRoot: string | null;
};
