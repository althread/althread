import { createEffect, onCleanup } from "solid-js";

/**
 * Handles Escape to exit maximized, and "f" or "F" to toggle maximized,
 * but only when not typing in an input, textarea, or contenteditable element.
 */
export function useGraphMaximizeHotkeys(
  setMaximized: (v: boolean | ((v: boolean) => boolean)) => void
) {
  createEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement;
      if (
        target.tagName === "INPUT" ||
        target.tagName === "TEXTAREA" ||
        target.isContentEditable
      ) {
        return; // Don't trigger hotkeys while typing in editor or input
      }
      if (e.key === "Escape") setMaximized(false);
      if (e.key === "f" || e.key === "F") { 
        setMaximized((v: boolean) => !v);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    onCleanup(() => window.removeEventListener("keydown", handleKeyDown));
  });
}