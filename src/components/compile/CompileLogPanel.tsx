import { useEffect, useMemo, useState } from "react";

import { parseCompileLog } from "../../compile/logParser";
import type { CompileRunState } from "../../state/compileState";

type CompileLogPanelProps = {
  runState: CompileRunState;
};

function countBySeverity(
  diagnostics: ReturnType<typeof parseCompileLog>,
  severity: "error" | "warning",
) {
  return diagnostics.filter((diagnostic) => diagnostic.severity === severity).length;
}

export default function CompileLogPanel({ runState }: CompileLogPanelProps) {
  const log = runState.result?.log ?? runState.error ?? "";
  const diagnostics = useMemo(() => parseCompileLog(log), [log]);
  const errorCount = countBySeverity(diagnostics, "error");
  const warningCount = countBySeverity(diagnostics, "warning");
  const [isOpen, setIsOpen] = useState(false);

  useEffect(() => {
    if (runState.status === "failure" || diagnostics.length > 0) {
      setIsOpen(true);
    }
  }, [diagnostics.length, runState.finishedAt, runState.status]);

  return (
    <section className="compile-log-panel" aria-label="Compile log">
      <button
        type="button"
        className="compile-log-toggle"
        aria-expanded={isOpen}
        onClick={() => setIsOpen((current) => !current)}
      >
        <span>Compile log</span>
        <span className="compile-log-summary">
          {errorCount} errors · {warningCount} warnings
        </span>
      </button>
      {isOpen ? (
        <div className="compile-log-body">
          <div className="compile-diagnostics">
            {diagnostics.length > 0 ? (
              diagnostics.map((diagnostic, index) => (
                <div
                  key={`${diagnostic.severity}-${diagnostic.file ?? "log"}-${diagnostic.line ?? "n"}-${index}`}
                  className={`compile-diagnostic compile-diagnostic-${diagnostic.severity}`}
                >
                  <span className="compile-diagnostic-severity">
                    {diagnostic.severity}
                  </span>
                  <span className="compile-diagnostic-message">
                    {diagnostic.message}
                  </span>
                  {diagnostic.file || diagnostic.line ? (
                    <span className="compile-diagnostic-location">
                      {[diagnostic.file, diagnostic.line ? `line ${diagnostic.line}` : null]
                        .filter(Boolean)
                        .join(": ")}
                    </span>
                  ) : null}
                </div>
              ))
            ) : (
              <div className="compile-diagnostics-empty">
                No parsed errors or warnings.
              </div>
            )}
          </div>
          <pre className="compile-log-raw">
            {log || "No compile log captured yet."}
          </pre>
        </div>
      ) : null}
    </section>
  );
}
