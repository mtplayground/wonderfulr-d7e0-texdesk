import type { FsEntry } from "./fs";

export type Template = {
  id: string;
  name: string;
  description: string;
  category: string;
  mainFileName: string;
  body: string;
  bibliography: string | null;
  isDefault: boolean;
  createdAt: string;
  updatedAt: string;
};

export type ApplyTemplateRequest = {
  workspaceRoot: string;
  targetDirectory: string;
  templateId: string;
  assignmentName: string;
};

export type AppliedTemplate = {
  mainFile: FsEntry;
  bibliographyFile: FsEntry | null;
};

export type TemplateInput = {
  id?: string;
  name: string;
  description: string;
  category: string;
  mainFileName: string;
  body: string;
  bibliography: string | null;
};

export type DeleteTemplateResult = {
  deletedId: string;
};
