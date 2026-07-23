import { useCallback, useEffect, useMemo, useRef, useState } from "react";

export type PaneLayout = {
  leftWidth: number;
  rightWidth: number;
};

type DragTarget = "left" | "right";

type DragState = {
  target: DragTarget;
};

const STORAGE_KEY = "texdesk:pane-layout";
const DEFAULT_LAYOUT: PaneLayout = {
  leftWidth: 360,
  rightWidth: 360,
};

const MIN_LEFT_WIDTH = 320;
const MIN_CENTER_WIDTH = 320;
const MIN_RIGHT_WIDTH = 240;
const RESIZER_WIDTH = 8;

function isPaneLayout(value: unknown): value is PaneLayout {
  if (!value || typeof value !== "object") {
    return false;
  }

  const candidate = value as Partial<PaneLayout>;
  return (
    typeof candidate.leftWidth === "number" &&
    Number.isFinite(candidate.leftWidth) &&
    typeof candidate.rightWidth === "number" &&
    Number.isFinite(candidate.rightWidth)
  );
}

function getStoredLayout(): PaneLayout {
  if (typeof window === "undefined") {
    return DEFAULT_LAYOUT;
  }

  let rawValue: string | null = null;
  try {
    rawValue = window.localStorage.getItem(STORAGE_KEY);
  } catch {
    return DEFAULT_LAYOUT;
  }

  if (!rawValue) {
    return DEFAULT_LAYOUT;
  }

  try {
    const parsed = JSON.parse(rawValue) as unknown;
    return isPaneLayout(parsed) ? parsed : DEFAULT_LAYOUT;
  } catch {
    return DEFAULT_LAYOUT;
  }
}

function clamp(value: number, min: number, max: number): number {
  if (max < min) {
    return min;
  }

  return Math.min(Math.max(value, min), max);
}

function clampLayout(layout: PaneLayout, containerWidth: number): PaneLayout {
  const availableWidth = Math.max(containerWidth - RESIZER_WIDTH * 2, 0);
  const leftMax = availableWidth - MIN_CENTER_WIDTH - MIN_RIGHT_WIDTH;
  const leftWidth = clamp(layout.leftWidth, MIN_LEFT_WIDTH, leftMax);
  const rightMax = availableWidth - leftWidth - MIN_CENTER_WIDTH;
  const rightWidth = clamp(layout.rightWidth, MIN_RIGHT_WIDTH, rightMax);

  return {
    leftWidth,
    rightWidth,
  };
}

export function usePaneLayout() {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const dragState = useRef<DragState | null>(null);
  const [layout, setLayout] = useState<PaneLayout>(() => getStoredLayout());

  const resizeFromPointer = useCallback((clientX: number, target: DragTarget) => {
    const container = containerRef.current;
    if (!container) {
      return;
    }

    const bounds = container.getBoundingClientRect();
    const pointerX = clientX - bounds.left;

    setLayout((current) => {
      const next =
        target === "left"
          ? { ...current, leftWidth: pointerX }
          : { ...current, rightWidth: bounds.width - pointerX };

      return clampLayout(next, bounds.width);
    });
  }, []);

  const beginResize = useCallback(
    (target: DragTarget, pointerId: number, clientX: number, element: HTMLElement) => {
      element.setPointerCapture(pointerId);
      dragState.current = { target };
      resizeFromPointer(clientX, target);
    },
    [resizeFromPointer],
  );

  const resizeHandlers = useMemo(
    () => ({
      onPointerMove(event: React.PointerEvent<HTMLDivElement>) {
        if (!dragState.current) {
          return;
        }

        event.preventDefault();
        resizeFromPointer(event.clientX, dragState.current.target);
      },
      onPointerUp(event: React.PointerEvent<HTMLDivElement>) {
        if (!dragState.current) {
          return;
        }

        dragState.current = null;
        if (event.currentTarget.hasPointerCapture(event.pointerId)) {
          event.currentTarget.releasePointerCapture(event.pointerId);
        }
      },
      onPointerCancel(event: React.PointerEvent<HTMLDivElement>) {
        dragState.current = null;
        if (event.currentTarget.hasPointerCapture(event.pointerId)) {
          event.currentTarget.releasePointerCapture(event.pointerId);
        }
      },
    }),
    [resizeFromPointer],
  );

  useEffect(() => {
    const container = containerRef.current;
    if (!container) {
      return undefined;
    }

    const resizeObserver = new ResizeObserver(([entry]) => {
      if (!entry) {
        return;
      }

      setLayout((current) => clampLayout(current, entry.contentRect.width));
    });

    resizeObserver.observe(container);
    return () => resizeObserver.disconnect();
  }, []);

  useEffect(() => {
    try {
      window.localStorage.setItem(STORAGE_KEY, JSON.stringify(layout));
    } catch {
      return;
    }
  }, [layout]);

  return {
    containerRef,
    layout,
    beginResize,
    resizeHandlers,
  };
}
