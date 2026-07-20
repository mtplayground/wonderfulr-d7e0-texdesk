import { useEffect, useState } from "react";

import { getAppConfig } from "../ipc/client";
import type { AppConfig } from "../types/config";

type ConfigState =
  | { status: "loading"; config: null; error: null }
  | { status: "ready"; config: AppConfig; error: null }
  | { status: "error"; config: null; error: string };

const initialConfigState: ConfigState = {
  status: "loading",
  config: null,
  error: null,
};

export function useAppConfig(): ConfigState {
  const [state, setState] = useState<ConfigState>(initialConfigState);

  useEffect(() => {
    let isMounted = true;

    getAppConfig()
      .then((config) => {
        if (isMounted) {
          setState({ status: "ready", config, error: null });
        }
      })
      .catch((error: unknown) => {
        if (!isMounted) {
          return;
        }

        const message = error instanceof Error ? error.message : String(error);
        setState({ status: "error", config: null, error: message });
      });

    return () => {
      isMounted = false;
    };
  }, []);

  return state;
}
