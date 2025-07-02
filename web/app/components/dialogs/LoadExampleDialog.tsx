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
            <i class="codicon codicon-file"></i>
            Load Example
          </div>
          <div class="confirmation-dialog-body">
            Where would you like to load the example?
          </div>
          <div class="confirmation-dialog-actions">
            <button class="button-secondary" onClick={props.onCancel}>
              Cancel
            </button>
            <button class="button-secondary" onClick={props.onLoadInCurrent}>
              Current File
            </button>
            <button class="button-primary" onClick={props.onLoadInNew}>
              New File
            </button>
          </div>
        </div>
      </div>
    </Show>
  );
};
