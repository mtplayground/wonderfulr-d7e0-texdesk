import { useEffect, useMemo, useState } from "react";

import {
  deleteTemplate,
  listTemplates,
  saveTemplate,
} from "../../ipc/client";
import type { Template, TemplateInput } from "../../types/templates";

type TemplateDraft = {
  id?: string;
  name: string;
  description: string;
  category: string;
  mainFileName: string;
  body: string;
  bibliography: string;
};

const EMPTY_DRAFT: TemplateDraft = {
  name: "",
  description: "",
  category: "custom",
  mainFileName: "assignment.tex",
  body: "\\documentclass[11pt]{article}\n\\begin{document}\n\n\\end{document}\n",
  bibliography: "",
};

function displayError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function draftFromTemplate(template: Template): TemplateDraft {
  return {
    id: template.id,
    name: template.name,
    description: template.description,
    category: template.category,
    mainFileName: template.mainFileName,
    body: template.body,
    bibliography: template.bibliography ?? "",
  };
}

function inputFromDraft(draft: TemplateDraft): TemplateInput {
  return {
    id: draft.id,
    name: draft.name,
    description: draft.description,
    category: draft.category,
    mainFileName: draft.mainFileName,
    body: draft.body,
    bibliography: draft.bibliography.trim() ? draft.bibliography : null,
  };
}

export default function TemplateManager() {
  const [templates, setTemplates] = useState<Template[]>([]);
  const [draft, setDraft] = useState<TemplateDraft>(EMPTY_DRAFT);
  const [isOpen, setIsOpen] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const activeTemplate = useMemo(
    () => templates.find((template) => template.id === draft.id) ?? null,
    [draft.id, templates],
  );

  async function loadTemplates() {
    setIsLoading(true);
    setError(null);
    try {
      const loaded = await listTemplates();
      setTemplates(loaded);
      if (!draft.id && loaded[0]) {
        setDraft(draftFromTemplate(loaded[0]));
      }
    } catch (loadError) {
      setError(displayError(loadError));
    } finally {
      setIsLoading(false);
    }
  }

  useEffect(() => {
    if (isOpen) {
      void loadTemplates();
    }
  }, [isOpen]);

  async function saveDraft(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setIsSaving(true);
    setError(null);
    try {
      const saved = await saveTemplate(inputFromDraft(draft));
      setTemplates((current) => {
        const existingIndex = current.findIndex((template) => template.id === saved.id);
        if (existingIndex === -1) {
          return [...current, saved].sort((left, right) =>
            left.name.localeCompare(right.name),
          );
        }

        const next = [...current];
        next[existingIndex] = saved;
        return next;
      });
      setDraft(draftFromTemplate(saved));
    } catch (saveError) {
      setError(displayError(saveError));
    } finally {
      setIsSaving(false);
    }
  }

  async function removeDraft() {
    if (!draft.id || !activeTemplate) {
      return;
    }

    if (!window.confirm(`Delete ${activeTemplate.name}?`)) {
      return;
    }

    setIsSaving(true);
    setError(null);
    try {
      await deleteTemplate(activeTemplate.id);
      const remaining = templates.filter((template) => template.id !== activeTemplate.id);
      setTemplates(remaining);
      setDraft(remaining[0] ? draftFromTemplate(remaining[0]) : EMPTY_DRAFT);
    } catch (deleteError) {
      setError(displayError(deleteError));
    } finally {
      setIsSaving(false);
    }
  }

  function updateDraft(field: keyof TemplateDraft, value: string) {
    setDraft((current) => ({ ...current, [field]: value }));
  }

  return (
    <section className="template-manager" aria-label="Templates">
      <button
        type="button"
        className="template-manager-toggle"
        onClick={() => setIsOpen((current) => !current)}
      >
        <span>Templates</span>
        <span>{isOpen ? "v" : ">"}</span>
      </button>
      {isOpen ? (
        <div className="template-manager-body">
          <div className="template-manager-actions">
            <button
              type="button"
              onClick={() => {
                setDraft(EMPTY_DRAFT);
                setError(null);
              }}
            >
              New
            </button>
            <button type="button" onClick={() => void loadTemplates()}>
              Reload
            </button>
            <button
              type="button"
              disabled={!draft.id || isSaving}
              onClick={() => void removeDraft()}
            >
              Delete
            </button>
          </div>
          {error ? <div className="template-manager-error">{error}</div> : null}
          <div className="template-list" aria-label="Template library">
            {isLoading ? <div className="template-empty">Loading</div> : null}
            {templates.map((template) => (
              <button
                key={template.id}
                type="button"
                className={`template-list-item${
                  draft.id === template.id ? " template-list-item-active" : ""
                }`}
                onClick={() => setDraft(draftFromTemplate(template))}
              >
                <span>{template.name}</span>
                <small>{template.category}</small>
              </button>
            ))}
          </div>
          <form className="template-editor" onSubmit={(event) => void saveDraft(event)}>
            <label>
              <span>Name</span>
              <input
                value={draft.name}
                onChange={(event) => updateDraft("name", event.currentTarget.value)}
              />
            </label>
            <label>
              <span>Category</span>
              <input
                value={draft.category}
                onChange={(event) => updateDraft("category", event.currentTarget.value)}
              />
            </label>
            <label>
              <span>Main file</span>
              <input
                value={draft.mainFileName}
                onChange={(event) =>
                  updateDraft("mainFileName", event.currentTarget.value)
                }
              />
            </label>
            <label>
              <span>Description</span>
              <textarea
                rows={2}
                value={draft.description}
                onChange={(event) =>
                  updateDraft("description", event.currentTarget.value)
                }
              />
            </label>
            <label>
              <span>Body</span>
              <textarea
                rows={8}
                spellCheck={false}
                value={draft.body}
                onChange={(event) => updateDraft("body", event.currentTarget.value)}
              />
            </label>
            <label>
              <span>Bibliography</span>
              <textarea
                rows={4}
                spellCheck={false}
                value={draft.bibliography}
                onChange={(event) =>
                  updateDraft("bibliography", event.currentTarget.value)
                }
              />
            </label>
            <button type="submit" disabled={isSaving}>
              {isSaving ? "Saving" : "Save template"}
            </button>
          </form>
        </div>
      ) : null}
    </section>
  );
}
