/** @jsxImportSource solid-js */
import { createEffect, createSignal, Show } from "solid-js";
import './GraphToolbar.css';

interface GraphToolbarProps {
  onFullscreen: () => void;
  onRecenter: () => void;
  onDownload: () => void;
  onDownloadCSV?: () => void;
  onDetails: () => void;
  isFullscreen?: boolean;
}

export default function GraphToolbar(props: GraphToolbarProps) {
  const [showDownloadMenu, setShowDownloadMenu] = createSignal(false);

  const handleDownloadClick = (e: MouseEvent) => {
    e.stopPropagation();
    setShowDownloadMenu(!showDownloadMenu());
  };

  const handlePNGDownload = (e: MouseEvent) => {
    e.stopPropagation();
    props.onDownload();
    setShowDownloadMenu(false);
  };

  const handleCSVDownload = (e: MouseEvent) => {
    e.stopPropagation();
    if (props.onDownloadCSV) {
      props.onDownloadCSV();
    }
    setShowDownloadMenu(false);
  };

  // Close menu when clicking outside
  const handleDocumentClick = () => {
    setShowDownloadMenu(false);
  };

  // Add event listener when menu is open
  createEffect(() => {
    if (showDownloadMenu()) {
      document.addEventListener('click', handleDocumentClick);
    } else {
      document.removeEventListener('click', handleDocumentClick);
    }
  });

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
      
      <div class="download-dropdown" style={{ position: 'relative' }}>
        <button title="Download" onClick={handleDownloadClick}>
          <i class="codicon codicon-desktop-download"></i>
        </button>
        
        <Show when={showDownloadMenu()}>
          <div class="download-menu">
            <button class="download-menu-item" onClick={handlePNGDownload}>
              <i class="codicon codicon-file-media"></i>
              Download as PNG
            </button>
            <Show when={props.onDownloadCSV}>
              <button class="download-menu-item" onClick={handleCSVDownload}>
                <i class="codicon codicon-file-text"></i>
                Download as CSV
              </button>
            </Show>
          </div>
        </Show>
      </div>

      <button title="Details" onClick={props.onDetails}>
        <i class="codicon codicon-info"></i>
      </button>
    </div>
  );
}