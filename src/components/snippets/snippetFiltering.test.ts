import assert from "node:assert/strict";
import test from "node:test";

import type { Snippet } from "../../types/snippets";
import { filterSnippets } from "./snippetFiltering";

const snippets: Snippet[] = [
  {
    id: "equation-align",
    name: "Aligned Equation",
    description: "Multi-line aligned equation block for derivations.",
    category: "equation",
    trigger: "align",
    body: "\\begin{align}\n\\end{align}\n",
    isDefault: true,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  },
  {
    id: "bibliography-article",
    name: "BibTeX Article Entry",
    description: "BibTeX article entry with common citation fields.",
    category: "bibliography",
    trigger: "bibarticle",
    body: "@article{key}\n",
    isDefault: true,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  },
];

test("returns all snippets for an empty query", () => {
  assert.deepEqual(filterSnippets(snippets, "   "), snippets);
});

test("matches snippets by name, category, trigger, and description", () => {
  assert.deepEqual(
    filterSnippets(snippets, "bib").map((snippet) => snippet.id),
    ["bibliography-article"],
  );
  assert.deepEqual(
    filterSnippets(snippets, "DERIVATIONS").map((snippet) => snippet.id),
    ["equation-align"],
  );
  assert.deepEqual(
    filterSnippets(snippets, "align").map((snippet) => snippet.id),
    ["equation-align"],
  );
});
