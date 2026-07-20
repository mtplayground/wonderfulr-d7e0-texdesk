import { useEffect, useRef, useState } from "react";

import { useAppConfig } from "../../config/useAppConfig";
import { useStoreStatus } from "../../config/useStoreStatus";
import {
  getWorkspaceState,
  rememberOpenFile,
  rememberWorkspaceRoot,
} from "../../ipc/client";
import { useCompileState } from "../../state/compileState";
import { useDocumentState } from "../../state/documentState";
import { usePaneLayout } from "../../state/layoutState";
import { useWorkspaceSync } from "../../state/workspaceSync";
import CodeMirrorEditor from "../editor/CodeMirrorEditor";
import FileTree from "../file-tree/FileTree";

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
  const appConfig = useAppConfig();
  const storeStatus = useStoreStatus();
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
        </header>
        <FileTree
          activePath={documentState.document?.path ?? null}
          onOpenFile={(path) => void documentState.openDocument(path)}
          refreshKey={workspaceSync.refreshKey}
          workspaceRoot={workspaceRoot}
        />
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
            <p className="pane-subtitle">{toolchainLabel}</p>
          </div>
        </header>
        <div className="preview-sheet" aria-label="PDF preview placeholder">
          <div className="preview-line preview-line-wide" />
          <div className="preview-line" />
          <div className="preview-line preview-line-short" />
          <div className="preview-block" />
          <div className="preview-line" />
          <div className="preview-line preview-line-wide" />
        </div>
      </aside>
    </main>
  );
}
