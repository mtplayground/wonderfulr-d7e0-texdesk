import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { displayUserError } from "../errors/appError";
import { readWorkspaceFile, writeWorkspaceFile } from "../ipc/client";

type OpenDocument = {
  path: string;
  contents: string;
  savedContents: string;
};

type DocumentStatus = "idle" | "loading" | "ready" | "saving" | "error";

export function useDocumentState(workspaceRoot: string | null) {
  const [document, setDocument] = useState<OpenDocument | null>(null);
  const [status, setStatus] = useState<DocumentStatus>("idle");
  const [error, setError] = useState<string | null>(null);
  const [externalChangePath, setExternalChangePath] = useState<string | null>(null);
  const lastLocalSave = useRef<{ path: string; savedAt: number } | null>(null);

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
        setExternalChangePath(null);
        setStatus("ready");
      } catch (openError) {
        setStatus("error");
        setError(displayUserError(openError, "filesystem"));
      }
    },
    [isDirty, workspaceRoot],
  );

  const updateContents = useCallback((contents: string) => {
    setDocument((current) => (current ? { ...current, contents } : current));
  }, []);

  const saveDocument = useCallback(async () => {
    if (!workspaceRoot || !document) {
      return false;
    }

    setStatus("saving");
    setError(null);
    try {
      await writeWorkspaceFile({
        workspaceRoot,
        path: document.path,
        contents: document.contents,
      });
      lastLocalSave.current = {
        path: document.path,
        savedAt: Date.now(),
      };
      setDocument((current) =>
        current
          ? {
              ...current,
              savedContents: current.contents,
            }
          : current,
      );
      setExternalChangePath(null);
      setStatus("ready");
      return true;
    } catch (saveError) {
      setStatus("error");
      setError(displayUserError(saveError, "filesystem"));
      return false;
    }
  }, [document, workspaceRoot]);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.defaultPrevented) {
        return;
      }

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
    setExternalChangePath(null);
  }, [workspaceRoot]);

  const handleWorkspaceChange = useCallback(
    (paths: string[]) => {
      if (!document || !paths.includes(document.path)) {
        return;
      }

      const localSave = lastLocalSave.current;
      if (
        localSave?.path === document.path &&
        Date.now() - localSave.savedAt < 1500
      ) {
        return;
      }

      setExternalChangePath(document.path);
    },
    [document],
  );

  return {
    document,
    error,
    externalChangePath,
    handleWorkspaceChange,
    isDirty,
    openDocument,
    saveDocument,
    status,
    updateContents,
  };
}
