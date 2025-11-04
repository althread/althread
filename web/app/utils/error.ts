export interface FormattedError {
  message: string,
  filePath?: string,
  line?: number,
  col?: number,
}

export function formatAlthreadError(e: any, fileContent?: string): FormattedError {
  if (!e) return { message: "Unknown error"};
  
  const lines: string[] = [];
  let filePath: string | undefined;
  let line: number | undefined;
  let col: number | undefined;

  // Position info
  if (e.pos) {
    filePath = e.pos.file_path || "unknown";
    line = e.pos.line;
    col = e.pos.col;

    lines.push(
      `File: ${filePath || "unknown"}\nLine: ${line}, Col: ${col}`
    );
    // Show error line and caret if fileContent is provided
    if (fileContent && typeof line === "number" && line > 0) {
      const fileLines = fileContent.split('\n');
      const errorLine = fileLines[line - 1] || "";
      const lineIndent = " ".repeat(line.toString().length);
      lines.push(`${lineIndent} |`);
      lines.push(`${line} | ${errorLine}`);
      lines.push(`${lineIndent} |${" ".repeat(col ? col : 0)}^---`);
      lines.push(`${lineIndent} |`);
    }
  }

  // Error type
  if (e.error_type) {
    lines.push(`Type: ${e.error_type}`);
  }

  // Message
  lines.push(`Message: ${e.message}`);

  // Stack trace
  if (e.stack && Array.isArray(e.stack) && e.stack.length > 0) {
    lines.push("\nStack trace:");
    e.stack.forEach((pos: any) => {
      lines.push(
        `  at ${pos.file_path || "unknown"}:${pos.line}:${pos.col}`
      );
    });
  }

  return {
    message: "ERROR:\n" + lines.join("\n"),
    filePath,
    line,
    col
  };
}

export function formatAlthreadErrorString(e: any, fileContent?: string): string {
  return formatAlthreadError(e, fileContent).message;
}