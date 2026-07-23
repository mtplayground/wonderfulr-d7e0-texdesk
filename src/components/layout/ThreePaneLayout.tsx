import { useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";

import { useAppConfig } from "../../config/useAppConfig";
import { useStoreStatus } from "../../config/useStoreStatus";
import { displayUserError } from "../../errors/appError";
import {
  getWorkspaceState,
  rememberOpenFile,
  rememberWorkspaceRoot,
} from "../../ipc/client";
import { useCompileState } from "../../state/compileState";
import { useDocumentState } from "../../state/documentState";
import { usePaneLayout } from "../../state/layoutState";
import { useWorkspaceSync } from "../../state/workspaceSync";
import CompileLogPanel from "../compile/CompileLogPanel";
import CodeMirrorEditor, {
  type CodeMirrorEditorHandle,
} from "../editor/CodeMirrorEditor";
import FileTree from "../file-tree/FileTree";
import PdfPreview from "../preview/PdfPreview";
import SnippetPicker from "../snippets/SnippetPicker";
import TemplateManager from "../templates/TemplateManager";

type CodedError = Error & {
  code: string;
};

function hasTauriRuntime(): boolean {
  return (
    typeof window !== "undefined" &&
    "__TAURI_INTERNALS__" in window &&
    Boolean((window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__)
  );
}

async function pickWorkspaceDirectory(): Promise<string | null> {
  if (!hasTauriRuntime()) {
    const error = new Error(
      "The native folder picker is only available in the desktop app.",
    ) as CodedError;
    error.name = "TauriUnavailableError";
    error.code = "tauri_unavailable";
    throw error;
  }

  return open({
    canCreateDirectories: false,
    directory: true,
    multiple: false,
    recursive: true,
    title: "Open Workspace Folder",
  });
}

function PaneResizer({
  label,
  onStart,
}: {
  label: string;
  onStart: (event: React.PointerEvent<HTMLButtonElement>) => void;
}) {
  return (
    <button
      className="pane-resizer"
      type="button"
      aria-label={label}
      onPointerDown={onStart}
    />
  );
}

export default function ThreePaneLayout() {
  const { containerRef, layout, beginResize, resizeHandlers } = usePaneLayout();
  const editorRef = useRef<CodeMirrorEditorHandle | null>(null);
  const appConfig = useAppConfig();
  const storeStatus = useStoreStatus();
  const [workspacePickerError, setWorkspacePickerError] = useState<string | null>(
    null,
  );
  const [restoreState, setRestoreState] = useState<{
    isReady: boolean;
    lastOpenFile: string | null;
    workspaceRoot: string | null;
  }>({
    isReady: false,
    lastOpenFile: null,
    workspaceRoot: null,
  });
  const didRestoreFile = useRef(false);
  const workspaceRoot = restoreState.workspaceRoot;
  const documentState = useDocumentState(workspaceRoot);
  const compileState = useCompileState({
    documentPath: documentState.document?.path ?? null,
    isDirty: documentState.isDirty,
    saveDocument: documentState.saveDocument,
    workspaceRoot,
  });
  const { handleWorkspaceChange } = documentState;
  const workspaceSync = useWorkspaceSync(workspaceRoot);
  const workspaceLabel =
    workspaceRoot
      ? workspaceRoot
      : "No workspace selected";
  const toolchainLabel =
    appConfig.status === "ready" && appConfig.config.latexToolchainPath
      ? appConfig.config.latexToolchainPath
      : "System LaTeX";
  const storeLabel =
    storeStatus.status === "ready" && storeStatus.store
      ? `Store v${storeStatus.store.schemaVersion}`
      : "Local store";
  const previewPdfPath = compileState.runState.result?.pdfPath ?? null;
  const previewRefreshKey = compileState.runState.finishedAt;

  async function handleOpenFolder() {
    setWorkspacePickerError(null);
    try {
      const selectedDirectory = await pickWorkspaceDirectory();
      if (!selectedDirectory) {
        return;
      }

      didRestoreFile.current = true;
      setRestoreState({
        isReady: true,
        lastOpenFile: null,
        workspaceRoot: selectedDirectory,
      });
    } catch (openError) {
      setWorkspacePickerError(displayUserError(openError, "filesystem"));
    }
  }

  useEffect(() => {
    if (appConfig.status !== "ready") {
      return;
    }

    let isMounted = true;
    const configWorkspaceRoot = appConfig.config.defaultWorkspaceRoot;
    getWorkspaceState()
      .then((state) => {
        if (!isMounted) {
          return;
        }

        setRestoreState({
          isReady: true,
          lastOpenFile: state?.lastOpenFile ?? null,
          workspaceRoot: state?.lastWorkspaceRoot ?? configWorkspaceRoot,
        });
      })
      .catch(() => {
        if (isMounted) {
          setRestoreState({
            isReady: true,
            lastOpenFile: null,
            workspaceRoot: configWorkspaceRoot,
          });
        }
      });

    return () => {
      isMounted = false;
    };
  }, [appConfig]);

  useEffect(() => {
    if (!restoreState.isReady || !workspaceRoot) {
      return;
    }

    rememberWorkspaceRoot(workspaceRoot).catch(() => undefined);
  }, [restoreState.isReady, workspaceRoot]);

  useEffect(() => {
    if (
      !restoreState.isReady ||
      !workspaceRoot ||
      !restoreState.lastOpenFile ||
      didRestoreFile.current
    ) {
      return;
    }

    didRestoreFile.current = true;
    void documentState.openDocument(restoreState.lastOpenFile);
  }, [documentState.openDocument, restoreState, workspaceRoot]);

  useEffect(() => {
    const path = documentState.document?.path;
    if (!workspaceRoot || !path) {
      return;
    }

    rememberOpenFile(workspaceRoot, path).catch(() => undefined);
  }, [documentState.document?.path, workspaceRoot]);

  useEffect(() => {
    if (workspaceSync.lastEvent) {
      handleWorkspaceChange(workspaceSync.lastEvent.paths);
    }
  }, [handleWorkspaceChange, workspaceSync.lastEvent]);

  return (
    <main
      ref={containerRef}
      className="workspace-shell"
      style={{
        gridTemplateColumns: `${layout.leftWidth}px 8px minmax(320px, 1fr) 8px ${layout.rightWidth}px`,
      }}
      {...resizeHandlers}
    >
      <aside className="workspace-pane workspace-pane-left" aria-label="File tree">
        <header className="pane-header">
          <div>
            <p className="pane-kicker">Files</p>
            <h1>Workspace</h1>
            <p className="pane-subtitle">{workspaceLabel}</p>
          </div>
          <button
            type="button"
            className="pane-header-action"
            onClick={() => void handleOpenFolder()}
          >
            Open Folder
          </button>
        </header>
        {workspacePickerError ? (
          <div className="workspace-picker-error">{workspacePickerError}</div>
        ) : null}
        {workspaceRoot ? (
          <>
            <FileTree
              activePath={documentState.document?.path ?? null}
              onOpenFile={(path) => void documentState.openDocument(path)}
              refreshKey={workspaceSync.refreshKey}
              workspaceRoot={workspaceRoot}
            />
            <TemplateManager />
          </>
        ) : (
          <section className="workspace-empty-state" aria-label="No workspace selected">
            <div>
              <p className="workspace-empty-kicker">Choose a workspace</p>
              <h2>Open a folder of LaTeX files</h2>
              <p>
                Pick an existing directory to populate the file tree and start
                editing your .tex documents.
              </p>
              <button
                type="button"
                className="workspace-empty-button"
                onClick={() => void handleOpenFolder()}
              >
                Open Folder
              </button>
            </div>
          </section>
        )}
      </aside>

      <PaneResizer
        label="Resize file tree pane"
        onStart={(event) =>
          beginResize("left", event.pointerId, event.clientX, event.currentTarget)
        }
      />

      <section className="workspace-pane workspace-pane-editor" aria-label="Editor">
        <header className="pane-header editor-header">
          <div>
            <p className="pane-kicker">Editor</p>
            <h2>{documentState.document?.path ?? "No document"}</h2>
            <p className="pane-subtitle">
              {documentState.externalChangePath
                ? "Changed on disk"
                : documentState.error ?? workspaceSync.error ?? storeLabel}
            </p>
          </div>
          <div className="editor-actions">
            <span className={`compile-status compile-status-${compileState.runState.status}`}>
              {compileState.statusLabel}
            </span>
            <button
              type="button"
              className="editor-compile-button"
              disabled={!compileState.canCompile}
              onClick={() => void compileState.compile()}
            >
              {compileState.runState.status === "running" ? "Compiling" : "Compile"}
            </button>
            <SnippetPicker
              disabled={!documentState.document || documentState.status === "loading"}
              onInsert={(snippet) => {
                editorRef.current?.insertSnippet(snippet.body);
              }}
            />
            <span className={`file-status${documentState.isDirty ? " is-dirty" : ""}`}>
              {documentState.isDirty ? "Unsaved" : "Saved"}
            </span>
            <button
              type="button"
              className="editor-save-button"
              disabled={!documentState.document || !documentState.isDirty}
              onClick={() => void documentState.saveDocument()}
            >
              Save
            </button>
          </div>
        </header>
        <CodeMirrorEditor
          ref={editorRef}
          ariaLabel="Document editor"
          disabled={!documentState.document || documentState.status === "loading"}
          value={documentState.document?.contents ?? ""}
          placeholderText="Select a .tex file"
          onChange={documentState.updateContents}
          onSave={() => void documentState.saveDocument()}
        />
        {documentState.externalChangePath ? (
          <div className="editor-sync-warning">
            Open file changed on disk.
          </div>
        ) : null}
        {compileState.runState.status === "success" && compileState.runState.result ? (
          <div className="compile-run-message compile-run-message-success">
            Compiled {compileState.runState.result.pdfPath}
          </div>
        ) : null}
        {compileState.runState.status === "failure" && compileState.runState.error ? (
          <div className="compile-run-message compile-run-message-failure">
            {compileState.runState.error}
          </div>
        ) : null}
        <CompileLogPanel runState={compileState.runState} />
      </section>

      <PaneResizer
        label="Resize preview pane"
        onStart={(event) =>
          beginResize("right", event.pointerId, event.clientX, event.currentTarget)
        }
      />

      <aside className="workspace-pane workspace-pane-preview" aria-label="Preview">
        <header className="pane-header">
          <div>
            <p className="pane-kicker">Preview</p>
            <h2>PDF</h2>
            <p className="pane-subtitle">{previewPdfPath ?? toolchainLabel}</p>
          </div>
        </header>
        <PdfPreview
          pdfPath={previewPdfPath}
          refreshKey={previewRefreshKey}
          workspaceRoot={workspaceRoot}
        />
      </aside>
    </main>
  );
}
