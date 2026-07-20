import { useAppConfig } from "../../config/useAppConfig";
import { useStoreStatus } from "../../config/useStoreStatus";
import { useDocumentState } from "../../state/documentState";
import { usePaneLayout } from "../../state/layoutState";
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
  const workspaceRoot =
    appConfig.status === "ready" ? appConfig.config.defaultWorkspaceRoot : null;
  const documentState = useDocumentState(workspaceRoot);
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
            <p className="pane-subtitle">{documentState.error ?? storeLabel}</p>
          </div>
          <div className="editor-actions">
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
        <textarea
          className="editor-textarea"
          aria-label="Document editor"
          disabled={!documentState.document || documentState.status === "loading"}
          value={documentState.document?.contents ?? ""}
          placeholder="Select a .tex file"
          spellCheck={false}
          onChange={(event) => documentState.updateContents(event.target.value)}
        />
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
