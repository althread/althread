/** @jsxImportSource solid-js */
import { Show } from 'solid-js';

export interface LoadExampleDialogProps {
  isOpen: boolean;
  onLoadInCurrent: () => void;
  onLoadInNew: () => void;
  onCancel: () => void;
}

export const LoadExampleDialog = (props: LoadExampleDialogProps) => {
  return (
    <Show when={props.isOpen}>
      <div class="confirmation-dialog-overlay" onClick={props.onCancel}>
        <div class="confirmation-dialog" onClick={(e) => e.stopPropagation()}>
          <div class="confirmation-dialog-header">
            <i class="codicon codicon-file-code"></i>
            Load Example
          </div>
          <div class="confirmation-dialog-body">
            Where would you like to load the example code?
            <br />
            <br />
            <div style="font-size: 12px; color: #858585;">
              Choose to replace the current file content or create a new file tab.
            </div>
          </div>
          <div class="confirmation-dialog-actions">
            <button class="button-secondary" onClick={props.onCancel}>
              <i class="codicon codicon-close"></i>
              Cancel
            </button>
            <button class="button-secondary" onClick={props.onLoadInCurrent}>
              <i class="codicon codicon-file"></i>
              Current File
            </button>
            <button class="button-primary" onClick={props.onLoadInNew}>
              <i class="codicon codicon-new-file"></i>
              New File
            </button>
          </div>
        </div>
      </div>
    </Show>
  );
};
