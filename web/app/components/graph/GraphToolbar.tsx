/** @jsxImportSource solid-js */

interface GraphToolbarProps {
  onFullscreen: () => void;
  onRecenter: () => void;
  onDownload: () => void;
  onDetails: () => void;
  isFullscreen?: boolean;
}

export default function GraphToolbar(props: GraphToolbarProps) {
  return (
    <div class="graph-toolbar">
      <button
        title={
          props.isFullscreen
            ? "Exit Fullscreen (Esc, F)"
            : "Fullscreen (F)"
        }
        onClick={props.onFullscreen}
      >
        <i class={props.isFullscreen ? "codicon codicon-chrome-close" : "codicon codicon-screen-full"}></i>
      </button>
      <button title="Recenter" onClick={props.onRecenter}>
        <i class="codicon codicon-record"></i>
      </button>
      <button title="Download" onClick={props.onDownload}>
        <i class="codicon codicon-desktop-download"></i>
      </button>
      <button title="Details" onClick={props.onDetails}>
        <i class="codicon codicon-info"></i>
      </button>
    </div>
  );
}