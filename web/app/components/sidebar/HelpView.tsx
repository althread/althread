/** @jsxImportSource solid-js */
import { createSignal, Show } from 'solid-js';
import { useNavigate } from '@solidjs/router';
import ExampleSearch from './ExampleSearch';
import './HelpView.css';

interface HelpViewProps {
  onLoadExample?: (content: string, fileName: string) => void;
}

export default function HelpView(props: HelpViewProps) {
  const navigate = useNavigate();
  const [showExamples, setShowExamples] = createSignal(false);

  return (
    <div class="help-view">
      <div class="help-view-header">
        <h3>Help & Resources</h3>
      </div>
      
      <div class="help-view-content">
        <Show when={!showExamples()}>
          <div class="help-links">
            <button class="help-link" onClick={() => setShowExamples(true)}>
              <i class="codicon codicon-file-code"></i>
              <div>
                <div class="help-link-title">Browse Examples</div>
                <div class="help-link-desc">Explore sample programs and code examples</div>
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
        </Show>
        
        <Show when={showExamples()}>
          <div class="example-search-container">
            <div class="example-search-header">
              <button class="back-button" onClick={() => setShowExamples(false)}>
                <i class="codicon codicon-arrow-left"></i>
                Back to Help
              </button>
            </div>
            <div class="example-search-content">
              <ExampleSearch 
                onLoadExample={(content, fileName) => {
                  props.onLoadExample?.(content, fileName);
                  setShowExamples(false); // Go back to help after loading
                }}
              />
            </div>
          </div>
        </Show>
      </div>
    </div>
  );
}
