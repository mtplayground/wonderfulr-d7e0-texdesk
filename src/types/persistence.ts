export type WorkspaceState = {
  lastWorkspaceRoot: string | null;
  lastOpenFile: string | null;
};

export type RecentProject = {
  workspaceRoot: string;
  lastOpenedAt: string;
};
