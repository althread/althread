export function formatAlthreadError(e: any, fileContent?: string): string {
  if (!e) return "Unknown error";
  const lines: string[] = [];

  // Position info
  if (e.pos) {
    lines.push(
      `File: ${e.pos.file_path || "unknown"}\nLine: ${e.pos.line}, Col: ${e.pos.col}`
    );
    // Show error line and caret if fileContent is provided
    if (fileContent && typeof e.pos.line === "number" && e.pos.line > 0) {
      const fileLines = fileContent.split('\n');
      const errorLine = fileLines[e.pos.line - 1] || "";
      const lineIndent = " ".repeat(e.pos.line.toString().length);
      lines.push(`${lineIndent} |`);
      lines.push(`${e.pos.line} | ${errorLine}`);
      lines.push(`${lineIndent} |${" ".repeat(e.pos.col)}^---`);
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

  return "ERROR:\n" + lines.join("\n");
}