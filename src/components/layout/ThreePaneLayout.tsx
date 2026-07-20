import { usePaneLayout } from "../../state/layoutState";

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
          </div>
        </header>
        <nav className="file-tree-preview" aria-label="Workspace files">
          <button type="button" className="tree-row tree-row-active">
            <span className="tree-icon" aria-hidden="true" />
            assignment.tex
          </button>
          <button type="button" className="tree-row">
            <span className="tree-icon tree-icon-folder" aria-hidden="true" />
            figures
          </button>
          <button type="button" className="tree-row tree-row-child">
            <span className="tree-icon" aria-hidden="true" />
            notes.bib
          </button>
        </nav>
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
            <h2>assignment.tex</h2>
          </div>
          <span className="file-status">Unsaved</span>
        </header>
        <div className="editor-surface" aria-label="Document editor placeholder">
          <pre>{String.raw`\documentclass{article}
\begin{document}

\section{Problem Set}

Start writing here.

\end{document}`}</pre>
        </div>
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
