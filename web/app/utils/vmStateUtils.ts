/**
 * VM State Utility Functions
 * 
 * Helper functions for working with VM state data.
 * Note: With typed data from Rust, most defensive type-checking is no longer needed.
 */

/**
 * Format a literal value for display
 * Handles strings, numbers, booleans, arrays, and objects
 */
export function literal(value: any): string {
    if (value === null || value === undefined) {
        return String(value);
    }
    
    if (typeof value === 'string') {
        return value;
    }
    
    if (typeof value === 'number' || typeof value === 'boolean') {
        return String(value);
    }
    
    if (Array.isArray(value)) {
        if (value.length === 0) return '[]';
        const items = value.map(v => literal(v)).join(', ');
        return `[${items}]`;
    }
    
    if (typeof value === 'object') {
        const entries = Object.entries(value);
        if (entries.length === 0) return '{}';
        const items = entries.map(([k, v]) => `${k}: ${literal(v)}`).join(', ');
        return `{${items}}`;
    }
    
    return String(value);
}

/**
 * Format a value as HTML with syntax highlighting
 */
export function literalHtml(value: any): string {
    if (value === null || value === undefined) {
        return `<span class="literal-null">${value}</span>`;
    }
    
    if (typeof value === 'string') {
        return `<span class="literal-string">"${escapeHtml(value)}"</span>`;
    }
    
    if (typeof value === 'number') {
        return `<span class="literal-number">${value}</span>`;
    }
    
    if (typeof value === 'boolean') {
        return `<span class="literal-boolean">${value}</span>`;
    }
    
    if (Array.isArray(value)) {
        if (value.length === 0) return '<span class="literal-array">[]</span>';
        const items = value.map(v => literalHtml(v)).join(', ');
        return `<span class="literal-array">[${items}]</span>`;
    }
    
    if (typeof value === 'object') {
        const entries = Object.entries(value);
        if (entries.length === 0) return '<span class="literal-object">{}</span>';
        const items = entries.map(([k, v]) => 
            `<span class="literal-key">${escapeHtml(k)}</span>: ${literalHtml(v)}`
        ).join(', ');
        return `<span class="literal-object">{${items}}</span>`;
    }
    
    return escapeHtml(String(value));
}

/**
 * Escape HTML special characters
 */
function escapeHtml(text: string): string {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

/**
 * Get a display name for a process/program
 */
export function getProcessDisplayName(pid: number, name?: string): string {
    return name || `PID ${pid}`;
}

/**
 * Format channel identifier
 */
export function formatChannelId(pid: number, channelName: string): string {
    return `${pid}:${channelName}`;
}

/**
 * Parse channel identifier back to components
 */
export function parseChannelId(channelId: string): { pid: number; name: string } | null {
    const match = channelId.match(/^(\d+):(.+)$/);
    if (!match) return null;
    return {
        pid: parseInt(match[1], 10),
        name: match[2],
    };
}
