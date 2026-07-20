import {
  HighlightStyle,
  LanguageSupport,
  StreamLanguage,
  defaultHighlightStyle,
  syntaxHighlighting,
} from "@codemirror/language";
import { stex } from "@codemirror/legacy-modes/mode/stex";
import { tags } from "@lezer/highlight";

export const latexLanguage = StreamLanguage.define(stex);

export const latexHighlightStyle = HighlightStyle.define([
  {
    tag: tags.keyword,
    color: "#1d4ed8",
    fontWeight: "700",
  },
  {
    tag: tags.atom,
    color: "#7c3aed",
  },
  {
    tag: tags.string,
    color: "#0f766e",
  },
  {
    tag: tags.comment,
    color: "#6b7280",
    fontStyle: "italic",
  },
  {
    tag: tags.bracket,
    color: "#475569",
  },
  {
    tag: tags.tagName,
    color: "#be123c",
    fontWeight: "700",
  },
]);

export function latexEditorExtensions() {
  return [
    new LanguageSupport(latexLanguage),
    syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
    syntaxHighlighting(latexHighlightStyle),
  ];
}
