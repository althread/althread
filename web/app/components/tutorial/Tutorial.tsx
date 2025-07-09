import { createSignal, createEffect, For } from 'solid-js';
import type { Component } from "solid-js";
import createEditor from '@components/editor/Editor';
import { marked } from 'marked';
import './Tutorial.css';
import Resizable from '@corvu/resizable';

import  { compile, run } from '../../../pkg/althread_web';

// Import all tutorials
import { tutorial as tutorialStep1 } from '@tutorials/TutorialStep1_Variables';
import { tutorial as tutorialStep2 } from '@tutorials/TutorialStep2_IfElse';
import { tutorial as tutorialStep3 } from '@tutorials/TutorialStep3_ForLoops';
import { tutorial as tutorialStep4 } from '@tutorials/TutorialStep4_Functions';
import { tutorial as tutorialStep5 } from '@tutorials/TutorialStep5_SharedBlocks';
import { tutorial as tutorialStep6 } from '@tutorials/TutorialStep6_Programs';
import { tutorial as tutorialStep7 } from '@tutorials/TutorialStep7_Wait';
import { tutorial as tutorialStep8 } from '@tutorials/TutorialStep8_Channels';
import { useNavigate } from '@solidjs/router';
import { Logo } from '@assets/images/Logo';

export interface TutorialStep {
  name: string; // e.g., "Variables", "IfElse"
  displayName: string; // e.g., "Step 1: Variables"
  content: string; // Markdown content
  defaultCode: string;
  validate: (code: string) => { success: boolean; message: string };
}

const tutorials: Record<string, TutorialStep> = {
  TutorialStep1_Variables: tutorialStep1,
  TutorialStep2_IfElse: tutorialStep2,
  TutorialStep3_ForLoops: tutorialStep3,
  TutorialStep4_Functions: tutorialStep4,
  TutorialStep5_SharedBlocks: tutorialStep5,
  TutorialStep6_Programs: tutorialStep6,
  TutorialStep7_Wait: tutorialStep7,
  TutorialStep8_Channels: tutorialStep8,
};

const tutorialOrder: string[] = [
  'TutorialStep1_Variables',
  'TutorialStep2_IfElse',
  'TutorialStep3_ForLoops',
  'TutorialStep4_Functions',
  'TutorialStep5_SharedBlocks',
  'TutorialStep6_Programs',
  'TutorialStep7_Wait',
  'TutorialStep8_Channels',
];

const Tutorial: Component = () => {
  const [code, setCode] = createSignal("");
  const [currentTutorialIndex, setCurrentTutorialIndex] = createSignal(0);
  const [activeTab, setActiveTab] = createSignal<'validation' | 'result'>('validation');
  const [validationOutput, setValidationOutput] = createSignal<{ message: string, success: boolean } | null>(null);
  const [executionResult, setExecutionResult] = createSignal<string>("");

  const getLocalStorageKey = (index: number): string => {
    if (index >= 0 && index < tutorialOrder.length) {
      const tutorialKey = tutorialOrder[index]; // This is the key like "TutorialStep1_Variables"
      const tutorial = tutorials[tutorialKey];
      if (tutorial && tutorial.name) {
        // Use the 'name' field (e.g., "Variables") for a more user-friendly key part
        return `althread_tutorial_${tutorial.name.replace(/\s+/g, '_')}`;
      }
      // Fallback if tutorial.name is not available (should not happen with current setup)
      console.warn(`Tutorial 'name' property not found for key ${tutorialKey}, using key itself for localStorage.`);
      return `althread_tutorial_${tutorialKey}`;
    }
    console.error("Attempted to get localStorage key with out-of-bounds index:", index);
    return `althread_tutorial_invalid_${index}`;
  };

  const navigate = useNavigate();

  // Effect to update content and code when currentTutorialIndex changes
  createEffect(() => {
    const tutorialKey = tutorialOrder[currentTutorialIndex()];
    const tutorialData = tutorials[tutorialKey];

    if (tutorialData) {
      const savedCode = localStorage.getItem(getLocalStorageKey(currentTutorialIndex()));
      const newCode = savedCode !== null ? savedCode : tutorialData.defaultCode;
      setCode(newCode); // This will trigger the editor update effect if editor is mounted
      setValidationOutput(null); // Clear validation from previous step
      setExecutionResult(""); // Clear execution result from previous step
      setActiveTab('validation'); // Reset to validation tab
    } else {
      console.error(`Tutorial data not found for key: ${tutorialKey}`);
      setCode(`// Error: Tutorial '${tutorialKey}' not found.`);
      setValidationOutput({ success: false, message: `Tutorial data for '${tutorialKey}' is missing.` });
    }
  });

  // Effect to save code to LocalStorage when it changes
  createEffect(() => {
    const currentCode = code();
    // Ensure tutorialOrder[currentTutorialIndex()] is valid before saving
    // and currentCode is not undefined (initial state might be)
    if (tutorialOrder[currentTutorialIndex()] && typeof currentCode === 'string') {
      localStorage.setItem(getLocalStorageKey(currentTutorialIndex()), currentCode);
    }
  });

  const compileFromEditor = (currentEditorCode: string) => {
    // Create a simple virtual filesystem with just the tutorial code
    const virtualFS = {
      'main.alt': currentEditorCode
    };
    return compile(currentEditorCode, virtualFS);
  };

  const editorInstance = createEditor({
    defaultValue: code(), // Initialize with current code signal
    onValueChange: (newCode) => {
      setCode(newCode); // Update signal, which triggers save effect
    },
    compile: compileFromEditor,
  });

  // Effect to synchronize the editor view if the code signal changes programmatically
  // (e.g., when changing tutorial steps and loading new default/saved code)
  createEffect(() => {
    const view = editorInstance.editorView();
    const signalCode = code();
    if (view && view.state.doc.toString() !== signalCode) {
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: signalCode }
      });
    }
  });

  const currentTutorial = (): TutorialStep | undefined => tutorials[tutorialOrder[currentTutorialIndex()]];

  const handleValidateCode = () => {
    const tutorial = currentTutorial();
    if (tutorial) {
      const result = tutorial.validate(code());
      setValidationOutput(result);
      setActiveTab('validation');
    } else {
      setValidationOutput({ success: false, message: "Error: Current tutorial not found for validation." });
    }
  };

  const handleRunCode = () => {
     const currentEditorCode = code();
     // Create a simple virtual filesystem with just the tutorial code
     const virtualFS = {
       'main.alt': currentEditorCode
     };
     const result = run(currentEditorCode, virtualFS);
     setExecutionResult(result.stdout.join('\n'));
     setActiveTab('result');
  };

  const handleNextTutorial = () => {
    if (currentTutorialIndex() < tutorialOrder.length - 1) {
      setCurrentTutorialIndex(currentTutorialIndex() + 1);
    } else {
      alert("Congratulations! You have completed all tutorial steps.");
    }
  };

  const handlePreviousTutorial = () => {
    if (currentTutorialIndex() > 0) {
      setCurrentTutorialIndex(currentTutorialIndex() - 1);
    }
  };

  return (
    <div class="tutorial-page-container">
      <div class="tutorial-header">
        <div class="brand">
            <Logo />
            <h3>Althread</h3>
        </div>
        <div class="tutorial-step-selector-wrapper">
            <select
                value={tutorialOrder[currentTutorialIndex()]}
                onChange={(e) => {
                const selectedStepKey = (e.currentTarget as HTMLSelectElement).value;
                const newIndex = tutorialOrder.indexOf(selectedStepKey);
                if (newIndex !== -1) {
                    setCurrentTutorialIndex(newIndex);
                }
                }}
            >
                <For each={tutorialOrder}>{(stepKey) => {
                const tutorial = tutorials[stepKey];
                return <option value={stepKey}>{tutorial ? tutorial.displayName : `Loading: ${stepKey}`}</option>;
                }}</For>
            </select>
        </div>
        {/* Placeholder for right-aligned actions if needed */}
        <div class="actions-placeholder">
          <button onClick={() => {
            navigate('/');
          }} class="vscode-button">
              <i class="codicon codicon-terminal-tmux"></i> Back to Editor
          </button>
        </div>
      </div>
      <Resizable id="tutorial-content-area">
        <Resizable.Panel class="explanation-pane" initialSize={0.4} minSize={0.2}>
            <div
            class="tutorial-content"
            innerHTML={currentTutorial()?.content ? marked(currentTutorial()!.content) as string : 'Loading tutorial content...'}
            ></div>
            <div class="navigation-buttons">
                <button class="vscode-button" onClick={handlePreviousTutorial} disabled={currentTutorialIndex() === 0}>
                    <i class="codicon codicon-arrow-left"></i> Previous
                </button>
                <button class="vscode-button" onClick={handleNextTutorial} disabled={currentTutorialIndex() === tutorialOrder.length - 1}>
                    Next <i class="codicon codicon-arrow-right"></i>
                </button>
            </div>
        </Resizable.Panel>
        <Resizable.Handle class="Resizable-handle"/>
        <Resizable.Panel class="editor-output-panes" initialSize={0.6} minSize={0.2}>
            <div class="editor-pane-area">
              <div class="editor-toolbar">
                <h3>Editor</h3>
                <div class="action-buttons-center editor-toolbar-actions">
                    <button onClick={handleValidateCode} class="vscode-button validate-button">
                        <i class="codicon codicon-check"></i> Validate
                    </button>
                    <button onClick={handleRunCode} class="vscode-button">
                        <i class="codicon codicon-play"></i> Run
                    </button>
                </div>
              </div>
              <div ref={editorInstance.ref} class="editor-instance-wrapper"></div>
            </div>
            <div class="output-pane">
                <div class="tab">
                    <button
                    class={`tab_button ${activeTab() === 'validation' ? 'active' : ''}`}
                    onClick={() => setActiveTab('validation')}
                    >
                    <h3>Validation</h3>
                    </button>
                    <button
                    class={`tab_button ${activeTab() === 'result' ? 'active' : ''}`}
                    onClick={() => setActiveTab('result')}
                    >
                    <h3>Result</h3>
                    </button>
                </div>
                <div class="tab-content">
                  <div class="console">
                    {activeTab() === 'validation' && (
                      <div class={`validation-message ${validationOutput() ? (validationOutput()!.success ? 'success' : 'error') : ''}`}>
                        {validationOutput()
                          ? validationOutput()!.message
                          : "No validation output yet."
                        }
                      </div>
                    )}
                    {activeTab() === 'result' && (
                      <div class="validation-message">
                        {executionResult()
                          ? executionResult()
                          : "No execution output yet."
                        }
                      </div>
                    )}
                  </div>
                </div>
            </div>
        </Resizable.Panel>
      </Resizable>
    </div>
  );
};

export default Tutorial;
