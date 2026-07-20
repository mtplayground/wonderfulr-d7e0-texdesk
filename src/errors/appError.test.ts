import assert from "node:assert/strict";
import test from "node:test";

import { displayUserError, type CodedError } from "./appError";

test("adds actionable guidance for a missing LaTeX toolchain", () => {
  const error = new Error("no LaTeX compiler found") as CodedError;
  error.code = "compile_toolchain_unavailable";

  assert.match(displayUserError(error, "compile"), /Install latexmk, pdflatex, or xelatex/);
  assert.match(displayUserError(error, "compile"), /no LaTeX compiler found/);
});

test("adds filesystem recovery guidance for missing paths", () => {
  const error = new Error("path was not found: course/main.tex") as CodedError;
  error.code = "fs_path_not_found";

  assert.match(displayUserError(error, "filesystem"), /Refresh the workspace tree/);
  assert.match(displayUserError(error, "filesystem"), /course\/main\.tex/);
});

test("falls back to context copy when there is no structured message", () => {
  assert.equal(displayUserError("", "compile"), "Compile failed. Review the log and try again.");
});
