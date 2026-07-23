import { useCallback, useEffect, useMemo, useState } from "react";

import { displayUserError } from "../../errors/appError";
import {
  applyTemplateToWorkspace,
  createWorkspaceDirectory,
  createWorkspaceFile,
  deleteWorkspaceEntry,
  listTemplates,
  listWorkspaceEntries,
  renameWorkspaceEntry,
} from "../../ipc/client";
import type { FsEntry } from "../../types/fs";
import type { Template } from "../../types/templates";

type FileTreeProps = {
  activePath: string | null;
  onOpenFile: (path: string) => void;
  refreshKey: number;
  workspaceRoot: string | null;
};

type EntriesByPath = Record<string, FsEntry[]>;

const ROOT_PATH = "";

function parentPath(path: string): string {
  const separatorIndex = path.lastIndexOf("/");
  return separatorIndex === -1 ? ROOT_PATH : path.slice(0, separatorIndex);
}

function joinPath(parent: string, child: string): string {
  return parent ? `${parent}/${child}` : child;
}

function entryParent(entry: FsEntry): string {
  return entry.kind === "directory" ? entry.path : parentPath(entry.path);
}

export default function FileTree({
  activePath,
  onOpenFile,
  refreshKey,
  workspaceRoot,
}: FileTreeProps) {
  const [entriesByPath, setEntriesByPath] = useState<EntriesByPath>({});
  const [expandedPaths, setExpandedPaths] = useState<Set<string>>(
    () => new Set([ROOT_PATH]),
  );
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const selectedEntry = useMemo(() => {
    if (!selectedPath) {
      return null;
    }

    return Object.values(entriesByPath)
      .flat()
      .find((entry) => entry.path === selectedPath) ?? null;
  }, [entriesByPath, selectedPath]);

  const selectedDirectory = selectedEntry ? entryParent(selectedEntry) : ROOT_PATH;

  const loadDirectory = useCallback(
    async (path: string) => {
      if (!workspaceRoot) {
        return;
      }

      setIsLoading(true);
      setError(null);
      try {
        const entries = await listWorkspaceEntries({
          workspaceRoot,
          path: path || undefined,
        });
        setEntriesByPath((current) => ({
          ...current,
          [path]: entries,
        }));
      } catch (loadError) {
        setError(displayUserError(loadError, "filesystem"));
      } finally {
        setIsLoading(false);
      }
    },
    [workspaceRoot],
  );

  useEffect(() => {
    setEntriesByPath({});
    setExpandedPaths(new Set([ROOT_PATH]));
    setSelectedPath(null);
    setError(null);

    if (workspaceRoot) {
      void loadDirectory(ROOT_PATH);
    }
  }, [loadDirectory, workspaceRoot]);

  useEffect(() => {
    if (!workspaceRoot || refreshKey === 0) {
      return;
    }

    for (const path of expandedPaths) {
      void loadDirectory(path);
    }
  }, [loadDirectory, refreshKey, workspaceRoot]);

  async function refreshParent(path: string) {
    await loadDirectory(path);
  }

  async function toggleDirectory(entry: FsEntry) {
    if (entry.kind !== "directory") {
      setSelectedPath(entry.path);
      if (entry.path.endsWith(".tex")) {
        onOpenFile(entry.path);
      }
      return;
    }

    setSelectedPath(entry.path);
    const nextExpanded = new Set(expandedPaths);
    if (nextExpanded.has(entry.path)) {
      nextExpanded.delete(entry.path);
      setExpandedPaths(nextExpanded);
      return;
    }

    nextExpanded.add(entry.path);
    setExpandedPaths(nextExpanded);
    await loadDirectory(entry.path);
  }

  async function createEntry(kind: "directory" | "file") {
    if (!workspaceRoot) {
      return;
    }

    const rawName = window.prompt(kind === "directory" ? "Folder name" : "File name");
    const name = rawName?.trim();
    if (!name) {
      return;
    }

    const targetPath = joinPath(selectedDirectory, name);
    setError(null);
    try {
      if (kind === "directory") {
        await createWorkspaceDirectory({ workspaceRoot, path: targetPath });
        setExpandedPaths((current) => new Set(current).add(targetPath));
      } else {
        await createWorkspaceFile({ workspaceRoot, path: targetPath });
      }
      await refreshParent(selectedDirectory);
    } catch (createError) {
      setError(displayUserError(createError, "filesystem"));
    }
  }

  async function createFromTemplate() {
    if (!workspaceRoot) {
      return;
    }

    setError(null);
    try {
      const availableTemplates = await listTemplates();
      if (availableTemplates.length === 0) {
        setError("No templates are available");
        return;
      }

      const templateChoice = window.prompt(
        `Template\n${availableTemplates
          .map((template, index) => `${index + 1}. ${template.name}`)
          .join("\n")}`,
        "1",
      );
      const selectedTemplate = templateFromChoice(availableTemplates, templateChoice);
      if (!selectedTemplate) {
        return;
      }

      const rawName = window.prompt(
        "Assignment file name",
        selectedTemplate.mainFileName,
      );
      const assignmentName = rawName?.trim();
      if (!assignmentName) {
        return;
      }

      const created = await applyTemplateToWorkspace({
        workspaceRoot,
        targetDirectory: selectedDirectory,
        templateId: selectedTemplate.id,
        assignmentName,
      });
      setSelectedPath(created.mainFile.path);
      setExpandedPaths((current) => new Set(current).add(selectedDirectory));
      await refreshParent(selectedDirectory);
      onOpenFile(created.mainFile.path);
    } catch (templateError) {
      setError(displayUserError(templateError, "filesystem"));
    }
  }

  async function renameSelected() {
    if (!workspaceRoot || !selectedEntry) {
      return;
    }

    const rawName = window.prompt("Rename", selectedEntry.name);
    const name = rawName?.trim();
    if (!name || name === selectedEntry.name) {
      return;
    }

    const fromPath = selectedEntry.path;
    const parent = parentPath(fromPath);
    const toPath = joinPath(parent, name);
    setError(null);
    try {
      const renamed = await renameWorkspaceEntry({ workspaceRoot, fromPath, toPath });
      setSelectedPath(renamed.path);
      setEntriesByPath((current) => {
        const next = { ...current };
        delete next[fromPath];
        return next;
      });
      setExpandedPaths((current) => {
        const next = new Set(current);
        if (next.delete(fromPath) && renamed.kind === "directory") {
          next.add(renamed.path);
        }
        return next;
      });
      await refreshParent(parent);
    } catch (renameError) {
      setError(displayUserError(renameError, "filesystem"));
    }
  }

  async function deleteSelected() {
    if (!workspaceRoot || !selectedEntry) {
      return;
    }

    const shouldDelete = window.confirm(`Delete ${selectedEntry.name}?`);
    if (!shouldDelete) {
      return;
    }

    const path = selectedEntry.path;
    const parent = parentPath(path);
    setError(null);
    try {
      await deleteWorkspaceEntry({ workspaceRoot, path });
      setSelectedPath(null);
      setEntriesByPath((current) => {
        const next = { ...current };
        delete next[path];
        return next;
      });
      setExpandedPaths((current) => {
        const next = new Set(current);
        next.delete(path);
        return next;
      });
      await refreshParent(parent);
    } catch (deleteError) {
      setError(displayUserError(deleteError, "filesystem"));
    }
  }

  function renderEntries(path: string, depth: number) {
    const entries = entriesByPath[path] ?? [];

    return entries.map((entry) => {
      const isDirectory = entry.kind === "directory";
      const isExpanded = expandedPaths.has(entry.path);
      const isSelected = selectedPath === entry.path || activePath === entry.path;

      return (
        <div className="tree-branch" key={entry.path}>
          <button
            type="button"
            className={`tree-row${isSelected ? " tree-row-active" : ""}`}
            style={{ paddingLeft: `${10 + depth * 16}px` }}
            aria-expanded={isDirectory ? isExpanded : undefined}
            title={entry.path}
            onClick={() => void toggleDirectory(entry)}
          >
            <span className="tree-caret" aria-hidden="true">
              {isDirectory ? (isExpanded ? "v" : ">") : ""}
            </span>
            <span
              className={`tree-icon${isDirectory ? " tree-icon-folder" : ""}`}
              aria-hidden="true"
            />
            <span className="tree-name">{entry.name}</span>
          </button>
          {isDirectory && isExpanded ? renderEntries(entry.path, depth + 1) : null}
        </div>
      );
    });
  }

  if (!workspaceRoot) {
    return <div className="file-tree-empty">No workspace selected</div>;
  }

  return (
    <div className="file-tree">
      <div className="file-tree-actions" aria-label="File actions">
        <button
          type="button"
          aria-label="New file"
          title="New file"
          onClick={() => void createEntry("file")}
        >
          New file
        </button>
        <button
          type="button"
          aria-label="New folder"
          title="New folder"
          onClick={() => void createEntry("directory")}
        >
          New folder
        </button>
        <button
          type="button"
          aria-label="New from template"
          title="New from template"
          onClick={() => void createFromTemplate()}
        >
          Template
        </button>
        <button
          type="button"
          aria-label="Rename selected item"
          title="Rename"
          disabled={!selectedEntry}
          onClick={() => void renameSelected()}
        >
          Rename
        </button>
        <button
          type="button"
          aria-label="Delete selected item"
          title="Delete"
          disabled={!selectedEntry}
          onClick={() => void deleteSelected()}
        >
          Delete
        </button>
      </div>
      {error ? <div className="file-tree-error">{error}</div> : null}
      <nav className="file-tree-list" aria-label="Workspace files">
        {isLoading && !entriesByPath[ROOT_PATH] ? (
          <div className="file-tree-empty">Loading</div>
        ) : null}
        {renderEntries(ROOT_PATH, 0)}
      </nav>
    </div>
  );
}

function templateFromChoice(
  templates: Template[],
  choice: string | null,
): Template | null {
  const trimmed = choice?.trim();
  if (!trimmed) {
    return null;
  }

  const selectedIndex = Number(trimmed);
  if (Number.isInteger(selectedIndex)) {
    return templates[selectedIndex - 1] ?? null;
  }

  return templates.find((template) => template.id === trimmed) ?? null;
}
