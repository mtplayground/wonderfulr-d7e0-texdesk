export type CodedError = Error & {
  code?: string;
};

export type ErrorContext = "compile" | "filesystem" | "generic";

const CODE_MESSAGES: Record<string, string> = {
  compile_file_expected: "Select a .tex source file before compiling.",
  compile_invalid_source_name: "The selected LaTeX source has an invalid file name.",
  compile_io: "Could not run the LaTeX process. Check file permissions and toolchain access.",
  compile_pdf_not_produced:
    "The compiler finished without producing a PDF. Review the compile log for the missing output reason.",
  compile_process_failed:
    "LaTeX reported a compile failure. Review the parsed errors and raw log, then re-run compile.",
  compile_toolchain_unavailable:
    "No LaTeX toolchain was found. Install latexmk, pdflatex, or xelatex, or set LATEX_TOOLCHAIN_PATH to the toolchain location.",
  fs_destination_exists:
    "A file or folder already exists at that path. Choose another name or delete the existing item first.",
  fs_directory_expected: "That action requires a folder, but the selected path is not a folder.",
  fs_file_expected: "That action requires a file, but the selected path is not a file.",
  fs_invalid_relative_path:
    "The path must stay inside the workspace and cannot be absolute or contain parent-directory segments.",
  fs_invalid_workspace_root:
    "The workspace root is missing or is not a folder. Choose an existing workspace directory.",
  fs_io: "The filesystem operation failed. Check permissions and whether the file is in use.",
  fs_path_not_found: "The file or folder no longer exists. Refresh the workspace tree and try again.",
  fs_path_outside_workspace:
    "The requested path is outside the workspace. Choose a file or folder inside the workspace root.",
};

function asErrorParts(error: unknown): { code: string | null; message: string } {
  if (error instanceof Error) {
    const coded = error as CodedError;
    return {
      code: coded.code ?? null,
      message: error.message,
    };
  }

  if (error && typeof error === "object") {
    const candidate = error as Partial<CodedError>;
    if (typeof candidate.message === "string") {
      return {
        code: typeof candidate.code === "string" ? candidate.code : null,
        message: candidate.message,
      };
    }
  }

  return { code: null, message: String(error) };
}

function contextFallback(context: ErrorContext): string {
  switch (context) {
    case "compile":
      return "Compile failed. Review the log and try again.";
    case "filesystem":
      return "Filesystem operation failed. Refresh the workspace and try again.";
    case "generic":
    default:
      return "Operation failed. Try again.";
  }
}

export function displayUserError(
  error: unknown,
  context: ErrorContext = "generic",
): string {
  const { code, message } = asErrorParts(error);
  const guidance = code ? CODE_MESSAGES[code] : null;
  const detail = message.trim();

  if (guidance && detail && detail !== guidance) {
    return `${guidance}\n${detail}`;
  }

  return guidance ?? (detail || contextFallback(context));
}
