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
    color: "#cc7832",
    fontWeight: "700",
  },
  {
    tag: tags.atom,
    color: "#9876aa",
  },
  {
    tag: tags.string,
    color: "#6a8759",
  },
  {
    tag: tags.comment,
    color: "#808080",
    fontStyle: "italic",
  },
  {
    tag: tags.bracket,
    color: "#a9b7c6",
  },
  {
    tag: tags.tagName,
    color: "#ffc66d",
    fontWeight: "700",
  },
]);

export function latexEditorExtensions() {
  return [
    new LanguageSupport(latexLanguage, [
      latexLanguage.data.of({
        commentTokens: {
          line: "%",
        },
      }),
    ]),
    syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
    syntaxHighlighting(latexHighlightStyle),
  ];
}
