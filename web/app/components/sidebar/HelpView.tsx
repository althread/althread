/** @jsxImportSource solid-js */
import { useNavigate } from '@solidjs/router';
import './HelpView.css';

interface HelpViewProps {
  onLoadExample?: () => void;
}

export default function HelpView(props: HelpViewProps) {
  const navigate = useNavigate();

  return (
    <div class="help-view">
      <div class="help-view-header">
        <h3>Help & Resources</h3>
      </div>
      
      <div class="help-view-content">
        <div class="help-links">
          <button class="help-link" onClick={() => props.onLoadExample?.()}>
            <i class="codicon codicon-file"></i>
            <div>
              <div class="help-link-title">Load Example</div>
              <div class="help-link-desc">Get started with a sample program</div>
            </div>
          </button>
          <button class="help-link" onClick={() => navigate('/tutorials')}>
            <i class="codicon codicon-book"></i>
            <div>
              <div class="help-link-title">Tutorials</div>
              <div class="help-link-desc">Step-by-step guides</div>
            </div>
          </button>
          <button class="help-link" onClick={() => window.open('https://althread.github.io/en/docs/guide/intro/', '_blank')}>
            <i class="codicon codicon-repo"></i>
            <div>
              <div class="help-link-title">Documentation</div>
              <div class="help-link-desc">Complete reference</div>
            </div>
          </button>
          <button class="help-link" onClick={() => window.open('https://github.com/althread/althread/issues/new', '_blank')}>
            <i class="codicon codicon-issue-opened"></i>
            <div>
              <div class="help-link-title">Report Issue</div>
              <div class="help-link-desc">Found a bug?</div>
            </div>
          </button>
        </div>
      </div>
    </div>
  );
}
