import { createSignal, createEffect, For } from 'solid-js';
import type { Component } from "solid-js";
import createEditor from '@components/editor/Editor';
import { marked } from 'marked';
import './Tutorial.css';
import Resizable from '@corvu/resizable';

import { workerClient } from '@utils/workerClient';
import { formatAlthreadError } from '@utils/error';

// Import all tutorials
import { tutorial as tutorialStep1 } from '@tutorials/TutorialStep1_Variables';
import { tutorial as tutorialStep2 } from '@tutorials/TutorialStep2_IfElse';
import { tutorial as tutorialStep3 } from '@tutorials/TutorialStep3_ForLoops';
import { tutorial as tutorialStep4 } from '@tutorials/TutorialStep4_Functions';
import { tutorial as tutorialStep5 } from '@tutorials/TutorialStep5_SharedBlocks';
import { tutorial as tutorialStep6 } from '@tutorials/TutorialStep6_Programs';
import { tutorial as tutorialStep7 } from '@tutorials/TutorialStep7_Wait';
import { tutorial as tutorialStep8 } from '@tutorials/TutorialStep8_Channels';
import { tutorial as tutorialStep9 } from '@tutorials/TutorialStep9_Imports';
import { useNavigate, useParams } from '@solidjs/router';
import { Logo } from '@assets/images/Logo';

// Helper function to create the virtual filesystem for the tutorial
const getTutorialVirtualFS = (code: string) => {
  return {
    'main.alt': code,
    'math.alt': `
      fn max(a: int, b: int) -> int {
        if a > b {
          return a;
        }
        return b;
      }
      `,
    'cool/fib.alt': `
      shared {
        let N: int = 8;
      }
      @private
      fn fibonacci_iterative(n: int, a: int, b: int) -> int {
        for i in 1..n {
          let c = a + b;
          a = b;
          b = c;
        }
        return b;
      }
      fn fibonacci_iterative_N() -> int {
        return fibonacci_iterative(N, 0, 1);      
      }
      `,
    'display.alt':`
      program Hello() {
        print("Hi there!");
      }
    `
  };
};

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
  TutorialStep9_Imports: tutorialStep9
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
  'TutorialStep9_Imports'
];

const Tutorial: Component = () => {
  const params = useParams();
  const navigate = useNavigate();

  const findIndexFromParam = (param: string | undefined): number => {
    if (!param) return 0; // Default to first tutorial
    const index = tutorialOrder.findIndex(key => tutorials[key]?.name.toLowerCase() === param.toLowerCase());
    return index !== -1 ? index : 0; // Default to first if param is invalid
  };

  const [code, setCode] = createSignal("");
  const [currentTutorialIndex, setCurrentTutorialIndex] = createSignal(findIndexFromParam(params.stepName));
  const [activeTab, setActiveTab] = createSignal<'validation' | 'result'>('validation');
  const [validationOutput, setValidationOutput] = createSignal<{ message: string, success: boolean } | null>(null);
  const [executionResult, setExecutionResult] = createSignal<string>("");
  const [executionError, setExecutionError] = createSignal<boolean>(false);

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

  // Effect to update the URL when the tutorial index changes
  createEffect(() => {
    const newIndex = currentTutorialIndex();
    const tutorialKey = tutorialOrder[newIndex];
    const tutorial = tutorials[tutorialKey];
    if (tutorial && tutorial.name) {
      // Only navigate if the URL doesn't already match to prevent loops
      if (params.stepName?.toLowerCase() !== tutorial.name.toLowerCase()) {
        navigate(`/tutorials/${tutorial.name}`, { replace: true });
      }
    }
  });

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

  // Function to create and render tutorial content with embedded read-only CodeMirror editors
  const renderTutorialContent = (markdownContent: string): string => {
    // First, process the markdown to find code blocks
    const codeBlockRegex = /```althread\n([\s\S]*?)\n```/g;
    let processedContent = markdownContent;
    const codeBlocks: { id: string; code: string }[] = [];
    
    // Extract code blocks and replace with placeholders
    let match;
    let blockIndex = 0;
    while ((match = codeBlockRegex.exec(markdownContent)) !== null) {
      const blockId = `code-block-${blockIndex}`;
      const code = match[1];
      codeBlocks.push({ id: blockId, code });
      
      // Replace the code block with a placeholder div
      processedContent = processedContent.replace(
        match[0],
        `<div id="${blockId}" class="tutorial-code-block"></div>`
      );
      blockIndex++;
    }
    
    // Convert markdown to HTML
    const htmlContent = marked(processedContent) as string;
    
    // After the HTML is rendered to the DOM, create CodeMirror editors
    setTimeout(() => {
      codeBlocks.forEach(({ id, code }) => {
        const container = document.getElementById(id);
        if (container) {
          // Clear any existing content
          container.innerHTML = '';
          
          // Create a read-only editor instance
          const readOnlyEditor = createEditor({
            defaultValue: code,
            onValueChange: undefined, // No change handler for read-only
            compile: () => {}, // No compilation needed for read-only examples
            filePath: 'example.alt'
          });
          
          // Create a wrapper div to mount the editor
          const editorWrapper = document.createElement('div');
          container.appendChild(editorWrapper);
          
          // Use the ref setter to mount the editor
          readOnlyEditor.ref(editorWrapper);
          
          // Set to read-only mode AFTER mounting
          setTimeout(() => {
            readOnlyEditor.setReadOnly(true);
            
            // Clear any selection that might occur on initialization
            const view = readOnlyEditor.safeEditorView();
            if (view) {
              view.dispatch({
                selection: { anchor: 0, head: 0 }
              });
            }
          }, 50); // Increased delay to ensure editor is fully mounted
        }
      });
    }, 10); // Small delay to ensure DOM is updated
    
    return htmlContent;
  };

  const compileFromEditor = async (currentEditorCode: string) => {
    const virtualFS = getTutorialVirtualFS(currentEditorCode);
    return await workerClient.compile(currentEditorCode, "main.alt", virtualFS);
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

  const handleRunCode = async () => {
    setExecutionError(false);
    const currentEditorCode = code();
    const virtualFS = getTutorialVirtualFS(currentEditorCode);
    console.log("Running code with virtual filesystem:", virtualFS);
    try {
      const result = await workerClient.run(currentEditorCode, "main.alt", virtualFS);
      setExecutionResult(result.stdout.join('\n'));
    } catch (e) {
      setExecutionError(true);
      setExecutionResult(formatAlthreadError(e, currentEditorCode));
    }
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
            innerHTML={currentTutorial()?.content ? renderTutorialContent(currentTutorial()!.content) : 'Loading tutorial content...'}
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
              <div ref={editorInstance.ref} class="editor-tutorial-instance-wrapper"></div>
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
                    class={`tab_button ${activeTab() === 'result' ? 'active' : ''} ${executionError() ? "execution-error" : ""}`}
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
                      <div class={`validation-message${executionError() ? " execution-error-box" : ""}`}>
                        <pre>
                          {executionResult() ? executionResult() : "No execution output yet."}
                        </pre>
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
