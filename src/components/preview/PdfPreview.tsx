import { useEffect, useMemo, useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import {
  GlobalWorkerOptions,
  getDocument,
  type PDFDocumentLoadingTask,
  type RenderTask,
} from "pdfjs-dist";
import pdfWorkerUrl from "pdfjs-dist/build/pdf.worker.mjs?url";

GlobalWorkerOptions.workerSrc = pdfWorkerUrl;

type PreviewStatus = "idle" | "loading" | "ready" | "error";

type PdfPreviewProps = {
  pdfPath: string | null;
  refreshKey: number | null;
  workspaceRoot: string | null;
};

function joinWorkspacePath(workspaceRoot: string, pdfPath: string) {
  const separator =
    workspaceRoot.endsWith("/") || workspaceRoot.endsWith("\\") ? "" : "/";
  return `${workspaceRoot}${separator}${pdfPath}`;
}

function displayError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export default function PdfPreview({
  pdfPath,
  refreshKey,
  workspaceRoot,
}: PdfPreviewProps) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [status, setStatus] = useState<PreviewStatus>("idle");
  const [error, setError] = useState<string | null>(null);
  const [pageCount, setPageCount] = useState(0);

  const pdfUrl = useMemo(() => {
    if (!workspaceRoot || !pdfPath) {
      return null;
    }

    const fileUrl = convertFileSrc(joinWorkspacePath(workspaceRoot, pdfPath));
    return `${fileUrl}${fileUrl.includes("?") ? "&" : "?"}v=${refreshKey ?? 0}`;
  }, [pdfPath, refreshKey, workspaceRoot]);

  useEffect(() => {
    const container = containerRef.current;
    if (!container || !pdfUrl) {
      setStatus("idle");
      setError(null);
      setPageCount(0);
      return;
    }

    const renderContainer = container;
    const sourceUrl = pdfUrl;
    let cancelled = false;
    let loadingTask: PDFDocumentLoadingTask | null = null;
    const renderTasks: RenderTask[] = [];

    async function renderPdf() {
      setStatus("loading");
      setError(null);
      setPageCount(0);
      renderContainer.replaceChildren();

      try {
        loadingTask = getDocument({ url: sourceUrl });
        const pdf = await loadingTask.promise;
        if (cancelled) {
          await pdf.destroy();
          return;
        }

        setPageCount(pdf.numPages);
        const availableWidth = Math.max(renderContainer.clientWidth - 28, 280);

        for (let pageNumber = 1; pageNumber <= pdf.numPages; pageNumber += 1) {
          if (cancelled) {
            break;
          }

          const page = await pdf.getPage(pageNumber);
          const unscaledViewport = page.getViewport({ scale: 1 });
          const scale = Math.min(Math.max(availableWidth / unscaledViewport.width, 0.4), 2);
          const viewport = page.getViewport({ scale });
          const canvas = document.createElement("canvas");
          const context = canvas.getContext("2d");
          if (!context) {
            throw new Error("Could not initialize PDF canvas.");
          }

          const outputScale = window.devicePixelRatio || 1;
          canvas.width = Math.floor(viewport.width * outputScale);
          canvas.height = Math.floor(viewport.height * outputScale);
          canvas.style.width = `${Math.floor(viewport.width)}px`;
          canvas.style.height = `${Math.floor(viewport.height)}px`;
          canvas.className = "pdf-preview-page";
          context.setTransform(outputScale, 0, 0, outputScale, 0, 0);

          renderContainer.appendChild(canvas);
          const renderTask = page.render({ canvasContext: context, viewport });
          renderTasks.push(renderTask);
          await renderTask.promise;
        }

        if (!cancelled) {
          setStatus("ready");
        }
        await pdf.destroy();
      } catch (renderError) {
        if (!cancelled) {
          setStatus("error");
          setError(displayError(renderError));
        }
      }
    }

    void renderPdf();

    return () => {
      cancelled = true;
      renderTasks.forEach((task) => task.cancel());
      if (loadingTask) {
        void loadingTask.destroy();
      }
    };
  }, [pdfUrl]);

  return (
    <div className="pdf-preview">
      {!pdfUrl ? (
        <div className="pdf-preview-empty">Compile a .tex file to preview its PDF.</div>
      ) : null}
      {pdfUrl && status === "loading" ? (
        <div className="pdf-preview-status">Loading PDF…</div>
      ) : null}
      {status === "error" && error ? (
        <div className="pdf-preview-error">{error}</div>
      ) : null}
      {status === "ready" ? (
        <div className="pdf-preview-status">{pageCount} page{pageCount === 1 ? "" : "s"}</div>
      ) : null}
      <div ref={containerRef} className="pdf-preview-pages" />
    </div>
  );
}
