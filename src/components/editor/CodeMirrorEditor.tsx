import {
  forwardRef,
  useEffect,
  useImperativeHandle,
  useMemo,
  useRef,
} from "react";
import {
  Compartment,
  EditorState,
  type Extension,
} from "@codemirror/state";
import {
  EditorView,
  crosshairCursor,
  drawSelection,
  dropCursor,
  highlightActiveLine,
  highlightActiveLineGutter,
  highlightSpecialChars,
  keymap,
  lineNumbers,
  placeholder,
  rectangularSelection,
  type ViewUpdate,
} from "@codemirror/view";
import {
  highlightSelectionMatches,
} from "@codemirror/search";
import { foldGutter } from "@codemirror/language";

import { texEditorErgonomics } from "../../editor/editorErgonomics";
import { latexEditorExtensions } from "../../editor/latexHighlighting";
import { applySnippetToText } from "../../editor/snippetInsertion";

type CodeMirrorEditorProps = {
  ariaLabel: string;
  disabled: boolean;
  onChange: (value: string) => void;
  onSave: () => void;
  placeholderText: string;
  value: string;
};

export type CodeMirrorEditorHandle = {
  insertSnippet: (body: string) => boolean;
};

const editorTheme = EditorView.theme({
  "&": {
    height: "100%",
    color: "#182235",
    backgroundColor: "#ffffff",
    fontSize: "0.95rem",
  },
  "&.cm-focused": {
    outline: "none",
  },
  ".cm-scroller": {
    fontFamily: '"SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace',
    lineHeight: "1.7",
  },
  ".cm-content": {
    minHeight: "100%",
    padding: "18px 18px 18px 0",
    caretColor: "#1d4ed8",
  },
  ".cm-line": {
    padding: "0 4px 0 12px",
  },
  ".cm-gutters": {
    backgroundColor: "#f1f4f8",
    color: "#718098",
    borderRight: "1px solid #d9e0ea",
  },
  ".cm-activeLineGutter": {
    backgroundColor: "#e5edf8",
    color: "#243047",
  },
  ".cm-activeLine": {
    backgroundColor: "#f5f8fc",
  },
  ".cm-selectionBackground, &.cm-focused .cm-selectionBackground": {
    backgroundColor: "#cfe0f7",
  },
  ".cm-placeholder": {
    color: "#607089",
  },
  "&.cm-editor-disabled": {
    color: "#607089",
  },
});

function createBaseExtensions(
  updateListener: (update: ViewUpdate) => void,
  ariaLabel: string,
  onSave: () => void,
): Extension[] {
  return [
    lineNumbers(),
    highlightActiveLineGutter(),
    highlightSpecialChars(),
    foldGutter(),
    drawSelection(),
    dropCursor(),
    EditorState.allowMultipleSelections.of(true),
    rectangularSelection(),
    crosshairCursor(),
    highlightActiveLine(),
    highlightSelectionMatches(),
    EditorView.lineWrapping,
    EditorView.contentAttributes.of({ "aria-label": ariaLabel }),
    EditorView.updateListener.of(updateListener),
    ...texEditorErgonomics({ onSave }),
    ...latexEditorExtensions(),
    editorTheme,
  ];
}

const CodeMirrorEditor = forwardRef<CodeMirrorEditorHandle, CodeMirrorEditorProps>(
  function CodeMirrorEditor(
    {
      ariaLabel,
      disabled,
      onChange,
      onSave,
      placeholderText,
      value,
    },
    ref,
  ) {
    const containerRef = useRef<HTMLDivElement | null>(null);
    const viewRef = useRef<EditorView | null>(null);
    const onChangeRef = useRef(onChange);
    const onSaveRef = useRef(onSave);
    const applyingExternalValue = useRef(false);
    const editableCompartment = useMemo(() => new Compartment(), []);
    const placeholderCompartment = useMemo(() => new Compartment(), []);

    useEffect(() => {
      onChangeRef.current = onChange;
    }, [onChange]);

    useEffect(() => {
      onSaveRef.current = onSave;
    }, [onSave]);

    useImperativeHandle(
      ref,
      () => ({
        insertSnippet(body: string) {
          const view = viewRef.current;
          if (!view || disabled) {
            return false;
          }

          const selection = view.state.selection.main;
          const insertion = applySnippetToText(
            view.state.doc.toString(),
            selection.from,
            selection.to,
            body,
          );
          view.dispatch({
            changes: {
              from: selection.from,
              to: selection.to,
              insert: insertion.inserted,
            },
            selection: {
              anchor: insertion.cursor,
            },
            scrollIntoView: true,
          });
          view.focus();
          return true;
        },
      }),
      [disabled],
    );

    useEffect(() => {
      const parent = containerRef.current;
      if (!parent) {
        return;
      }

      const updateListener = (update: ViewUpdate) => {
        if (!update.docChanged || applyingExternalValue.current) {
          return;
        }

        onChangeRef.current(update.state.doc.toString());
      };

      const view = new EditorView({
        parent,
        state: EditorState.create({
          doc: value,
          extensions: [
            ...createBaseExtensions(updateListener, ariaLabel, () => onSaveRef.current()),
            editableCompartment.of([
              EditorView.editable.of(!disabled),
              EditorState.readOnly.of(disabled),
            ]),
            placeholderCompartment.of(placeholder(placeholderText)),
          ],
        }),
      });

      viewRef.current = view;

      return () => {
        view.destroy();
        viewRef.current = null;
      };
    }, [ariaLabel, editableCompartment, placeholderCompartment]);

    useEffect(() => {
      const view = viewRef.current;
      if (!view) {
        return;
      }

      view.dispatch({
        effects: editableCompartment.reconfigure([
          EditorView.editable.of(!disabled),
          EditorState.readOnly.of(disabled),
        ]),
      });
    }, [disabled, editableCompartment]);

    useEffect(() => {
      const view = viewRef.current;
      if (!view) {
        return;
      }

      const currentValue = view.state.doc.toString();
      if (currentValue === value) {
        return;
      }

      try {
        applyingExternalValue.current = true;
        view.dispatch({
          changes: {
            from: 0,
            to: view.state.doc.length,
            insert: value,
          },
        });
      } finally {
        applyingExternalValue.current = false;
      }
    }, [value]);

    useEffect(() => {
      const view = viewRef.current;
      if (!view) {
        return;
      }

      view.dispatch({
        effects: placeholderCompartment.reconfigure(placeholder(placeholderText)),
      });
    }, [placeholderCompartment, placeholderText]);

    return (
      <div
        ref={containerRef}
        className={`codemirror-editor${disabled ? " codemirror-editor-disabled" : ""}`}
      />
    );
  },
);

export default CodeMirrorEditor;
