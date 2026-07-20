import { useEffect, useMemo, useRef, useState } from "react";

import { listSnippets } from "../../ipc/client";
import type { Snippet } from "../../types/snippets";
import { filterSnippets } from "./snippetFiltering";

type SnippetPickerProps = {
  disabled: boolean;
  onInsert: (snippet: Snippet) => void;
};

function displayError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export default function SnippetPicker({
  disabled,
  onInsert,
}: SnippetPickerProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [snippets, setSnippets] = useState<Snippet[]>([]);
  const [query, setQuery] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const panelRef = useRef<HTMLDivElement | null>(null);
  const buttonRef = useRef<HTMLButtonElement | null>(null);
  const queryRef = useRef<HTMLInputElement | null>(null);

  const filteredSnippets = useMemo(() => {
    return filterSnippets(snippets, query);
  }, [query, snippets]);

  async function loadSnippets() {
    setIsLoading(true);
    setError(null);
    try {
      setSnippets(await listSnippets());
    } catch (loadError) {
      setError(displayError(loadError));
    } finally {
      setIsLoading(false);
    }
  }

  useEffect(() => {
    if (isOpen) {
      void loadSnippets();
      window.setTimeout(() => queryRef.current?.focus(), 0);
    }
  }, [isOpen]);

  useEffect(() => {
    if (!isOpen) {
      return;
    }

    function handlePointerDown(event: PointerEvent) {
      const target = event.target;
      if (!(target instanceof Node)) {
        return;
      }

      if (
        panelRef.current?.contains(target) ||
        buttonRef.current?.contains(target)
      ) {
        return;
      }

      setIsOpen(false);
    }

    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        setIsOpen(false);
        buttonRef.current?.focus();
      }
    }

    window.addEventListener("pointerdown", handlePointerDown);
    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("pointerdown", handlePointerDown);
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [isOpen]);

  function insertSnippet(snippet: Snippet) {
    onInsert(snippet);
    setIsOpen(false);
    setQuery("");
  }

  return (
    <div className="snippet-picker">
      <button
        ref={buttonRef}
        type="button"
        className="editor-snippet-button"
        disabled={disabled}
        aria-expanded={isOpen}
        onClick={() => setIsOpen((current) => !current)}
      >
        Snippets
      </button>
      {isOpen ? (
        <div ref={panelRef} className="snippet-picker-panel">
          <div className="snippet-picker-search">
            <input
              ref={queryRef}
              value={query}
              placeholder="Find snippet"
              onChange={(event) => setQuery(event.currentTarget.value)}
            />
            <button type="button" onClick={() => void loadSnippets()}>
              Reload
            </button>
          </div>
          {error ? <div className="snippet-picker-error">{error}</div> : null}
          <div className="snippet-picker-list" aria-label="Snippet library">
            {isLoading ? <div className="snippet-picker-empty">Loading</div> : null}
            {!isLoading && filteredSnippets.length === 0 ? (
              <div className="snippet-picker-empty">No snippets</div>
            ) : null}
            {filteredSnippets.map((snippet) => (
              <button
                key={snippet.id}
                type="button"
                className="snippet-picker-item"
                onClick={() => insertSnippet(snippet)}
              >
                <span>{snippet.name}</span>
                <small>
                  {snippet.category}
                  {snippet.trigger ? ` / ${snippet.trigger}` : ""}
                </small>
              </button>
            ))}
          </div>
        </div>
      ) : null}
    </div>
  );
}
