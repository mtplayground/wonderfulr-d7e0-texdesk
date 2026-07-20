import { useCallback, useEffect, useMemo, useState } from "react";

import { readWorkspaceFile, writeWorkspaceFile } from "../ipc/client";

type OpenDocument = {
  path: string;
  contents: string;
  savedContents: string;
};

type DocumentStatus = "idle" | "loading" | "ready" | "saving" | "error";

function displayError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function useDocumentState(workspaceRoot: string | null) {
  const [document, setDocument] = useState<OpenDocument | null>(null);
  const [status, setStatus] = useState<DocumentStatus>("idle");
  const [error, setError] = useState<string | null>(null);

  const isDirty = useMemo(
    () => Boolean(document && document.contents !== document.savedContents),
    [document],
  );

  const openDocument = useCallback(
    async (path: string) => {
      if (!workspaceRoot || !path.endsWith(".tex")) {
        return;
      }

      if (isDirty && !window.confirm("Discard unsaved changes?")) {
        return;
      }

      setStatus("loading");
      setError(null);
      try {
        const file = await readWorkspaceFile({ workspaceRoot, path });
        setDocument({
          path: file.path,
          contents: file.contents,
          savedContents: file.contents,
        });
        setStatus("ready");
      } catch (openError) {
        setStatus("error");
        setError(displayError(openError));
      }
    },
    [isDirty, workspaceRoot],
  );

  const updateContents = useCallback((contents: string) => {
    setDocument((current) => (current ? { ...current, contents } : current));
  }, []);

  const saveDocument = useCallback(async () => {
    if (!workspaceRoot || !document) {
      return;
    }

    setStatus("saving");
    setError(null);
    try {
      await writeWorkspaceFile({
        workspaceRoot,
        path: document.path,
        contents: document.contents,
      });
      setDocument((current) =>
        current
          ? {
              ...current,
              savedContents: current.contents,
            }
          : current,
      );
      setStatus("ready");
    } catch (saveError) {
      setStatus("error");
      setError(displayError(saveError));
    }
  }, [document, workspaceRoot]);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (!(event.metaKey || event.ctrlKey) || event.key.toLowerCase() !== "s") {
        return;
      }

      event.preventDefault();
      void saveDocument();
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [saveDocument]);

  useEffect(() => {
    setDocument(null);
    setStatus("idle");
    setError(null);
  }, [workspaceRoot]);

  return {
    document,
    error,
    isDirty,
    openDocument,
    saveDocument,
    status,
    updateContents,
  };
}
