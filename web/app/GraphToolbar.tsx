/** @jsxImportSource solid-js */

interface GraphToolbarProps {
  onFullscreen: () => void;
  onRecenter: () => void;
  // You could add more props here if you add more generic buttons later
}

export default function GraphToolbar(props: GraphToolbarProps) {
  return (
    <div class="graph-toolbar">
      <button title="Fullscreen" onClick={props.onFullscreen}>
        <i class="codicon codicon-screen-full"></i>
      </button>
      <button title="Recenter" onClick={props.onRecenter}>
        <i class="codicon codicon-refresh"></i>
      </button>
    </div>
  );
}