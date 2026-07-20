import { useEffect, useState } from "react";

import { getStoreStatus } from "../ipc/client";
import type { StoreStatus } from "../types/store";

type StoreStatusState =
  | { status: "loading"; store: null }
  | { status: "ready"; store: StoreStatus | null };

const initialStoreStatusState: StoreStatusState = {
  status: "loading",
  store: null,
};

export function useStoreStatus(): StoreStatusState {
  const [state, setState] = useState<StoreStatusState>(initialStoreStatusState);

  useEffect(() => {
    let isMounted = true;

    getStoreStatus().then((store) => {
      if (isMounted) {
        setState({ status: "ready", store });
      }
    });

    return () => {
      isMounted = false;
    };
  }, []);

  return state;
}
