import type { Snippet } from "../../types/snippets";

export function filterSnippets(snippets: Snippet[], query: string): Snippet[] {
  const normalizedQuery = query.trim().toLowerCase();
  if (!normalizedQuery) {
    return snippets;
  }

  return snippets.filter((snippet) =>
    [
      snippet.name,
      snippet.description,
      snippet.category,
      snippet.trigger,
    ].some((value) => value.toLowerCase().includes(normalizedQuery)),
  );
}
