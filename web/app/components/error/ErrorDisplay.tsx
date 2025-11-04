/** @jsxImportSource solid-js */
import type { FormattedError } from "@utils/error";
import './ErrorDisplay.css';

interface ErrorDisplayProps {
    error: FormattedError;
    onFileClick?: (filePath: string) => void;
}

export default function ErrorDisplay(props: ErrorDisplayProps) {
    const formatErrorWithClickableFile = () => {
        if (!props.error.filePath) {
            return <pre>{props.error.message}</pre>
        }

        const message = props.error.message;
        const lines = message.split('\n');
        
        return (
            <pre>
                {lines.map((line, index) => {
                    // Handle main file path line
                    if (line.startsWith('File: ')) {
                        const filePath = line.substring(6); // Remove "File: " prefix
                        return (
                            <span>
                                File: <button
                                    class="file-link"
                                    onClick={() => props.onFileClick && props.onFileClick(filePath)}
                                    title={`Click to open ${filePath}`}
                                >
                                    {filePath}
                                </button>
                                {index < lines.length - 1 && '\n'}
                            </span>
                        );
                    }
                    
                    // Handle stack trace lines: "  at filename:line:col"
                    const stackTraceMatch = line.match(/^(\s*at\s+)([^:]+):(\d+):(\d+)$/);
                    if (stackTraceMatch) {
                        const [, prefix, filePath, lineNum, colNum] = stackTraceMatch;
                        return (
                            <span>
                                {prefix}
                                <button
                                    class="file-link"
                                    onClick={() => props.onFileClick && props.onFileClick(filePath)}
                                    title={`Click to open ${filePath} at line ${lineNum}`}
                                >
                                    {filePath}
                                </button>
                                :{lineNum}:{colNum}
                                {index < lines.length - 1 && '\n'}
                            </span>
                        );
                    }
                    
                    // Regular line - no clickable elements
                    return (
                        <span>
                            {line}
                            {index < lines.length - 1 && '\n'}
                        </span>
                    );
                })}
            </pre>
        );
    };

    return (
        <div class="error-display">
            {formatErrorWithClickableFile()}
        </div>
    );
}