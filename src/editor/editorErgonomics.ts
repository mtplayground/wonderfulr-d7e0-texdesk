import {
  EditorState,
  Prec,
  type Extension,
} from "@codemirror/state";
import { keymap } from "@codemirror/view";
import {
  defaultKeymap,
  history,
  historyKeymap,
  indentWithTab,
  toggleLineComment,
} from "@codemirror/commands";
import {
  bracketMatching,
  foldKeymap,
  indentOnInput,
} from "@codemirror/language";
import {
  closeBrackets,
  closeBracketsKeymap,
  completionKeymap,
} from "@codemirror/autocomplete";
import { searchKeymap } from "@codemirror/search";

type EditorErgonomicsOptions = {
  onSave: () => void;
};

export function texEditorErgonomics({
  onSave,
}: EditorErgonomicsOptions): Extension[] {
  return [
    EditorState.tabSize.of(2),
    history(),
    indentOnInput(),
    bracketMatching({
      afterCursor: true,
      brackets: "()[]{}",
      maxScanDistance: 10_000,
    }),
    closeBrackets(),
    Prec.highest(
      keymap.of([
        {
          key: "Mod-s",
          preventDefault: true,
          run: () => {
            onSave();
            return true;
          },
        },
        {
          key: "Mod-/",
          preventDefault: true,
          run: toggleLineComment,
        },
        indentWithTab,
      ]),
    ),
    keymap.of([
      ...closeBracketsKeymap,
      ...defaultKeymap,
      ...searchKeymap,
      ...historyKeymap,
      ...foldKeymap,
      ...completionKeymap,
    ]),
  ];
}
