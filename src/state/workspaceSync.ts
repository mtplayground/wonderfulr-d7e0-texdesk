import { useEffect, useState } from "react";

import { displayUserError } from "../errors/appError";
import {
  onWorkspaceChanged,
  startWorkspaceWatcher,
  stopWorkspaceWatcher,
} from "../ipc/client";
import type { WorkspaceChangeEvent } from "../types/sync";

type WorkspaceSyncState = {
  error: string | null;
  lastEvent: WorkspaceChangeEvent | null;
  refreshKey: number;
};

export function useWorkspaceSync(workspaceRoot: string | null): WorkspaceSyncState {
  const [state, setState] = useState<WorkspaceSyncState>({
    error: null,
    lastEvent: null,
    refreshKey: 0,
  });

  useEffect(() => {
    let isMounted = true;
    let unlisten: (() => void) | null = null;

    setState({ error: null, lastEvent: null, refreshKey: 0 });
    if (!workspaceRoot) {
      return undefined;
    }

    onWorkspaceChanged((event) => {
      if (!isMounted) {
        return;
      }

      setState((current) => ({
        error: null,
        lastEvent: event,
        refreshKey: current.refreshKey + 1,
      }));
    })
      .then((unsubscribe) => {
        unlisten = unsubscribe;
      })
      .catch((subscribeError: unknown) => {
        if (isMounted) {
          setState((current) => ({
            ...current,
            error: displayUserError(subscribeError, "filesystem"),
          }));
        }
      });

    startWorkspaceWatcher(workspaceRoot).catch((watchError: unknown) => {
      if (isMounted) {
        setState((current) => ({
          ...current,
          error: displayUserError(watchError, "filesystem"),
        }));
      }
    });

    return () => {
      isMounted = false;
      if (unlisten) {
        unlisten();
      }
      stopWorkspaceWatcher().catch(() => undefined);
    };
  }, [workspaceRoot]);

  return state;
}
