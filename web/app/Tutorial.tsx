import { createSignal, createEffect, For } from 'solid-js';
import type { Component } from "solid-js";
import createEditor from './Editor';
import { marked } from 'marked';
import './Tutorial.css';

// Import all tutorials
import { tutorial as tutorialStep1 } from './tutorials/TutorialStep1_Variables';
import { tutorial as tutorialStep2 } from './tutorials/TutorialStep2_IfElse';
import { tutorial as tutorialStep3 } from './tutorials/TutorialStep3_ForLoops';
import { tutorial as tutorialStep4 } from './tutorials/TutorialStep4_SharedBlocks';
import { tutorial as tutorialStep5 } from './tutorials/TutorialStep5_Programs';
import { tutorial as tutorialStep6 } from './tutorials/TutorialStep6_Wait';
import { tutorial as tutorialStep7 } from './tutorials/TutorialStep7_Channels';

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
  TutorialStep4_SharedBlocks: tutorialStep4,
  TutorialStep5_Programs: tutorialStep5,
  TutorialStep6_Wait: tutorialStep6,
  TutorialStep7_Channels: tutorialStep7,
};

const tutorialOrder: string[] = [
  'TutorialStep1_Variables',
  'TutorialStep2_IfElse',
  'TutorialStep3_ForLoops',
  'TutorialStep4_SharedBlocks',
  'TutorialStep5_Programs',
  'TutorialStep6_Wait',
  'TutorialStep7_Channels',
];

const Tutorial: Component = () => {
  const [code, setCode] = createSignal("");
  // const [executionResult, setExecutionResult] = createSignal(""); // For "Result" tab if implemented
  const [currentTutorialIndex, setCurrentTutorialIndex] = createSignal(0);
  const [activeTab, setActiveTab] = createSignal<'validation' | 'result'>('validation');
  const [validationOutput, setValidationOutput] = createSignal<{ message: string, success: boolean } | null>(null);

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

  // Effect to update content and code when currentTutorialIndex changes
  createEffect(() => {
    const tutorialKey = tutorialOrder[currentTutorialIndex()];
    const tutorialData = tutorials[tutorialKey];

    if (tutorialData) {
      const savedCode = localStorage.getItem(getLocalStorageKey(currentTutorialIndex()));
      const newCode = savedCode !== null ? savedCode : tutorialData.defaultCode;
      setCode(newCode); // This will trigger the editor update effect if editor is mounted
      setValidationOutput(null); // Clear validation from previous step
      // setExecutionResult(""); // Clear execution result from previous step
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
    // Placeholder for actual compilation/execution logic for the "Result" tab
    // console.log("Compile/Execute called with:", currentEditorCode);
    // setExecutionResult("Simulated execution output.");
    return []; // Assuming this is for diagnostics or similar, adjust as needed
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
    <div class="tutorial-container">
      <div class="explanation-pane">
        <div class="tutorial-header">
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
        <div
          class="tutorial-content"
          innerHTML={currentTutorial()?.content ? marked(currentTutorial()!.content) as string : 'Loading tutorial content...'}
        ></div>
        <div class="navigation-buttons">
          <button onClick={handlePreviousTutorial} disabled={currentTutorialIndex() === 0}>
            &larr; Previous
          </button>
          <button onClick={handleValidateCode} class="validate-button">
            Validate Code
          </button>
          <button onClick={handleNextTutorial} disabled={currentTutorialIndex() === tutorialOrder.length - 1}>
            Next &rarr;
          </button>
        </div>
      </div>
      <div class="editor-output-panes">
        <div class="editor-pane-area">
          <h3>Editor</h3>
          <div ref={editorInstance.ref} class="editor-instance-wrapper"></div>
        </div>
        <div class="output-pane">
          <div class="output-tabs">
            <button
              class={activeTab() === 'validation' ? 'active' : ''}
              onClick={() => setActiveTab('validation')}
            >
              Validation Output
            </button>
            <button
              class={activeTab() === 'result' ? 'active' : ''}
              onClick={() => setActiveTab('result')}
            >
              Result (Placeholder)
            </button>
          </div>
          <div class="output-content">
            {activeTab() === 'validation' && validationOutput() && (
              <div class={`validation-message ${validationOutput()?.success ? 'success' : 'error'}`}>
                {validationOutput()?.message}
              </div>
            )}
            {activeTab() === 'result' && (
              <div>
                {/* Placeholder for actual execution result */}
                {/* To use executionResult signal: executionResult() */}
                Execution results will appear here.
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};

export default Tutorial;
