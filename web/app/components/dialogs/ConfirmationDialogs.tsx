/** @jsxImportSource solid-js */
import { Show, For } from 'solid-js';

// Move Confirmation Dialog Component
export const MoveConfirmationDialog = (props: {
  isOpen: boolean;
  title: string;
  message: string;
  fileName: string;
  onConfirm: () => void;
  onCancel: () => void;
}) => {
  return (
    <Show when={props.isOpen}>
      <div class="confirmation-dialog-overlay" onClick={props.onCancel}>
        <div class="confirmation-dialog" onClick={(e) => e.stopPropagation()}>
          <div class="confirmation-dialog-header">
            <i class="codicon codicon-warning"></i>
            {props.title}
          </div>
          <div class="confirmation-dialog-body">
            {props.message}
            <br />
            <span class="confirmation-dialog-file-name">{props.fileName}</span>
          </div>
          <div class="confirmation-dialog-actions">
            <button class="button-secondary" onClick={props.onCancel}>
              Cancel
            </button>
            <button class="button-primary" onClick={props.onConfirm}>
              Replace
            </button>
          </div>
        </div>
      </div>
    </Show>
  );
};

// Delete Confirmation Dialog Component
export const DeleteConfirmationDialog = (props: {
  isOpen: boolean;
  paths: string[];
  onConfirm: () => void;
  onCancel: () => void;
}) => {
  const getDeleteMessage = () => {
    const count = props.paths.length;
    if (count === 1) {
      return (
        <>
          Are you sure you want to delete <span class="confirmation-dialog-file-name">{props.paths[0]}</span>?
        </>
      );
    } else {
      return `Are you sure you want to delete ${count} items?`;
    }
  };

  return (
    <Show when={props.isOpen}>
      <div class="confirmation-dialog-overlay" onClick={props.onCancel}>
        <div class="confirmation-dialog" onClick={(e) => e.stopPropagation()}>
          <div class="confirmation-dialog-header">
            <i class="codicon codicon-trash"></i>
            Delete {props.paths.length === 1 ? 'Item' : 'Items'}
          </div>
          <div class="confirmation-dialog-body">
            {getDeleteMessage()}
            <Show when={props.paths.length > 1}>
              <br />
              <br />
              <div style="max-height: 120px; overflow-y: auto; font-size: 12px; color: #999;">
                <For each={props.paths}>
                  {(path) => <div>• <span class="confirmation-dialog-file-name">{path}</span></div>}
                </For>
              </div>
            </Show>
          </div>
          <div class="confirmation-dialog-actions">
            <button class="button-secondary" onClick={props.onCancel}>
              Cancel
            </button>
            <button class="button-primary" onClick={props.onConfirm} style="background-color: #c74e39;">
              Delete
            </button>
          </div>
        </div>
      </div>
    </Show>
  );
};
