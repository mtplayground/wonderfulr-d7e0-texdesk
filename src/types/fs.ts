export type FsEntryKind = "directory" | "file";

export type FsEntry = {
  name: string;
  path: string;
  kind: FsEntryKind;
  sizeBytes: number | null;
  modifiedMs: number | null;
};

export type FileContent = {
  path: string;
  contents: string;
};

export type ListWorkspaceRequest = {
  workspaceRoot: string;
  path?: string;
};

export type WorkspacePathRequest = {
  workspaceRoot: string;
  path: string;
};

export type WriteFileRequest = WorkspacePathRequest & {
  contents: string;
};

export type CreateFileRequest = WorkspacePathRequest & {
  contents?: string;
};

export type RenameEntryRequest = {
  workspaceRoot: string;
  fromPath: string;
  toPath: string;
};

export type DeleteResult = {
  deletedPath: string;
};
