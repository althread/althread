/** @jsxImportSource solid-js */

export interface EmptyEditorProps {
  onNewFile: () => void;
}

export const EmptyEditor = (props: EmptyEditorProps) => {
  return (
    <div class="empty-editor">
      <div class="empty-editor-content">
        <div class="empty-editor-icon">
          <i class="codicon codicon-new-file"></i>
        </div>
        <h3>No file is open</h3>
        <p>Create a new file to get started</p>
        <button 
          class="vscode-button primary" 
          onClick={props.onNewFile}
        >
          <i class="codicon codicon-new-file"></i>
          New File
        </button>
      </div>
    </div>
  );
};
