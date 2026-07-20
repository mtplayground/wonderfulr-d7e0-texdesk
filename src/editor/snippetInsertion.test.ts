import assert from "node:assert/strict";
import test from "node:test";

import {
  applySnippetToText,
  normalizeSnippetBody,
} from "./snippetInsertion";

test("normalizes snippet bodies with a trailing newline", () => {
  assert.equal(normalizeSnippetBody("\\alpha"), "\\alpha\n");
  assert.equal(normalizeSnippetBody("\\beta\n"), "\\beta\n");
});

test("applies a snippet at the cursor and reports the next cursor", () => {
  const result = applySnippetToText("before after", 7, 7, "\\begin{align}\n\\end{align}");

  assert.equal(
    result.value,
    "before \\begin{align}\n\\end{align}\nafter",
  );
  assert.equal(result.cursor, "before \\begin{align}\n\\end{align}\n".length);
});

test("replaces the selected range when inserting a snippet", () => {
  const result = applySnippetToText("A selected value", 2, 10, "x &= y");

  assert.equal(result.value, "A x &= y\n value");
  assert.equal(result.inserted, "x &= y\n");
});
