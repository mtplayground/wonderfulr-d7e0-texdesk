import { useCallback, useEffect, useMemo, useState } from "react";

import { compileDocument } from "../ipc/client";
import type { CompileResult } from "../types/compile";

export type CompileStatus = "idle" | "running" | "success" | "failure";

type CompileRunState = {
  error: string | null;
  finishedAt: number | null;
  result: CompileResult | null;
  startedAt: number | null;
  status: CompileStatus;
};

const INITIAL_RUN_STATE: CompileRunState = {
  error: null,
  finishedAt: null,
  result: null,
  startedAt: null,
  status: "idle",
};

function displayError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function useCompileState({
  documentPath,
  isDirty,
  saveDocument,
  workspaceRoot,
}: {
  documentPath: string | null;
  isDirty: boolean;
  saveDocument: () => Promise<boolean>;
  workspaceRoot: string | null;
}) {
  const [runState, setRunState] = useState<CompileRunState>(INITIAL_RUN_STATE);

  const canCompile = Boolean(
    workspaceRoot && documentPath && runState.status !== "running",
  );

  const compile = useCallback(async () => {
    if (!workspaceRoot || !documentPath || runState.status === "running") {
      return;
    }

    const startedAt = Date.now();
    setRunState({
      error: null,
      finishedAt: null,
      result: null,
      startedAt,
      status: "running",
    });

    try {
      if (isDirty) {
        const saved = await saveDocument();
        if (!saved) {
          throw new Error("Save failed before compile.");
        }
      }

      const result = await compileDocument({
        workspaceRoot,
        path: documentPath,
      });
      setRunState({
        error: null,
        finishedAt: Date.now(),
        result,
        startedAt,
        status: "success",
      });
    } catch (compileError) {
      setRunState({
        error: displayError(compileError),
        finishedAt: Date.now(),
        result: null,
        startedAt,
        status: "failure",
      });
    }
  }, [documentPath, isDirty, runState.status, saveDocument, workspaceRoot]);

  useEffect(() => {
    setRunState(INITIAL_RUN_STATE);
  }, [documentPath, workspaceRoot]);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.defaultPrevented) {
        return;
      }

      if (!(event.metaKey || event.ctrlKey) || event.key !== "Enter") {
        return;
      }

      event.preventDefault();
      void compile();
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [compile]);

  const statusLabel = useMemo(() => {
    switch (runState.status) {
      case "running":
        return "Compiling";
      case "success":
        return "Compiled";
      case "failure":
        return "Failed";
      case "idle":
      default:
        return "Ready";
    }
  }, [runState.status]);

  return {
    canCompile,
    compile,
    runState,
    statusLabel,
  };
}
