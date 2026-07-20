export type SnippetInsertion = {
  cursor: number;
  inserted: string;
  value: string;
};

export function normalizeSnippetBody(body: string): string {
  return body.endsWith("\n") ? body : `${body}\n`;
}

export function applySnippetToText(
  value: string,
  from: number,
  to: number,
  body: string,
): SnippetInsertion {
  const start = Math.max(0, Math.min(from, value.length));
  const end = Math.max(start, Math.min(to, value.length));
  const inserted = normalizeSnippetBody(body);

  return {
    cursor: start + inserted.length,
    inserted,
    value: `${value.slice(0, start)}${inserted}${value.slice(end)}`,
  };
}
