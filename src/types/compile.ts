export type CompileStrategy = "latexmk" | "manual-passes";

export type CompileToolchain = {
  strategy: CompileStrategy;
  engine: string;
  bibliographyTool: string | null;
};

export type CompileDocumentRequest = {
  workspaceRoot: string;
  path: string;
};

export type CompileResult = {
  pdfPath: string;
  log: string;
  toolchain: CompileToolchain;
};
