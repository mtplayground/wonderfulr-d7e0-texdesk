export type CompileDiagnosticSeverity = "error" | "warning";

export type CompileDiagnostic = {
  file: string | null;
  line: number | null;
  message: string;
  severity: CompileDiagnosticSeverity;
};

const FILE_LINE_PATTERN = /^(.+?\.tex):(\d+):\s*(.+)$/;
const LATEX_WARNING_PATTERN = /^(LaTeX|Package|Class)\s+(.+?\s+)?Warning:\s*(.+)$/;
const BOX_WARNING_PATTERN = /^(Over|Under)full \\[hv]box .+?(?:lines?\s+(\d+)(?:--\d+)?)?/;

function cleanMessage(message: string) {
  return message.replace(/\s+/g, " ").trim();
}

function diagnosticKey(diagnostic: CompileDiagnostic) {
  return [
    diagnostic.severity,
    diagnostic.file ?? "",
    diagnostic.line ?? "",
    diagnostic.message,
  ].join("|");
}

export function parseCompileLog(log: string): CompileDiagnostic[] {
  const diagnostics: CompileDiagnostic[] = [];
  const seen = new Set<string>();
  const lines = log.split(/\r?\n/);

  function addDiagnostic(diagnostic: CompileDiagnostic) {
    const message = cleanMessage(diagnostic.message);
    if (!message) {
      return;
    }

    const normalized = { ...diagnostic, message };
    const key = diagnosticKey(normalized);
    if (seen.has(key)) {
      return;
    }

    seen.add(key);
    diagnostics.push(normalized);
  }

  lines.forEach((line, index) => {
    const fileLineMatch = line.match(FILE_LINE_PATTERN);
    if (fileLineMatch) {
      addDiagnostic({
        file: fileLineMatch[1],
        line: Number.parseInt(fileLineMatch[2], 10),
        message: fileLineMatch[3],
        severity: "error",
      });
      return;
    }

    if (line.startsWith("! ")) {
      addDiagnostic({
        file: null,
        line: null,
        message: line.slice(2),
        severity: "error",
      });
      return;
    }

    const warningMatch = line.match(LATEX_WARNING_PATTERN);
    if (warningMatch) {
      addDiagnostic({
        file: null,
        line: null,
        message: warningMatch[3],
        severity: "warning",
      });
      return;
    }

    const boxWarningMatch = line.match(BOX_WARNING_PATTERN);
    if (boxWarningMatch) {
      addDiagnostic({
        file: null,
        line: boxWarningMatch[2] ? Number.parseInt(boxWarningMatch[2], 10) : null,
        message: line,
        severity: "warning",
      });
      return;
    }

    if (line.includes("There were undefined references")) {
      addDiagnostic({
        file: null,
        line: null,
        message: line,
        severity: "warning",
      });
      return;
    }

    if (line.includes("Citation") && line.includes("undefined")) {
      addDiagnostic({
        file: null,
        line: null,
        message: line,
        severity: "warning",
      });
      return;
    }

    if (line.startsWith("l.") && index > 0) {
      addDiagnostic({
        file: null,
        line: Number.parseInt(line.slice(2), 10) || null,
        message: `${lines[index - 1]} ${line}`,
        severity: "error",
      });
    }
  });

  return diagnostics;
}
