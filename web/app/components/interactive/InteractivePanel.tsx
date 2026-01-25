import { createEffect, createSignal, Show } from "solid-js";
import Resizable from '@corvu/resizable'
import { renderMessageFlowGraph } from "@components/graph/CommGraph";
import { rendervmStates } from "@components/graph/vmStatesDisplay";
import VMStateInspector from "@components/graph/VMStateInspector";
import type { NextStateOption } from "../../types/vm-state";
import './InteractivePanel.css';

interface InteractivePanelProps {
  isVisible: boolean;
  interactiveStates: NextStateOption[];
  currentVMState: any;
  isFinished: boolean;
  executionOutput: string;
  executionError: boolean;
  onExecuteStep: (index: number) => void;
  onClose: () => void;
  onReset: () => void;
  // Add the same props that the main app uses
  stdout?: string;
  commGraphOut?: any[];
  vmStates?: any[];
  isRun?: boolean;
  // Interactive-specific data
  interactiveMessageFlow?: any[];
  interactiveVmStates?: any[];
  interactiveStepLines?: number[][];
  editor?: any;
}

export default function InteractivePanel(props: InteractivePanelProps) {
  const [isMinimized, setIsMinimized] = createSignal(false);
  const [activeTab, setActiveTab] = createSignal<'choices' | 'state'>('choices');
  const [rightPanelTab, setRightPanelTab] = createSignal<'console' | 'execution' | 'msg_flow' | 'vm_states'>('console');
  
  // Ref for execution console to enable auto-scrolling
  let executionConsoleRef: HTMLDivElement | undefined;
  
  // Auto-scroll execution console to bottom when content changes
  createEffect(() => {
    if (rightPanelTab() === "execution" && executionConsoleRef && props.executionOutput) {
      executionConsoleRef.scrollTop = executionConsoleRef.scrollHeight;
    }
  });

  const toggleMinimize = () => {
    setIsMinimized(!isMinimized());
  };

  const handleExecutionTabClick = (tab: 'console' | 'execution' | 'msg_flow' | 'vm_states') => {
    setRightPanelTab(tab);
  };

  // Helper function to get source code lines from the editor
  const getSourceLines = (lines: number[]): string[] => {
    if (!props.editor || !lines || lines.length === 0) {
      return [];
    }
    
    try {
      const doc = props.editor.editorView().state.doc;
      const sourceLines = lines.map(lineNum => {
        if (lineNum > 0 && lineNum <= doc.lines) {
          return doc.line(lineNum).text.trim();
        }
        return "";
      }).filter(line => line !== "");
      
      // Remove duplicates while preserving order
      return [...new Set(sourceLines)];
    } catch (e) {
      return [];
    }
  };

  // Helper to format instructions with message delivery info
  const formatInstructions = (state: NextStateOption): JSX.Element => {
    const lines = getSourceLines(state.lines);
    
    // Check if this is a message operation
    const hasMessageOp = state.instructions.some(inst => 
      inst.includes('SEND') || inst.includes('RECEIVE') || inst.includes('DELIVER')
    );
    
    if (lines.length > 0) {
      return (
        <>
          {lines.map((line, i) => (
            <>
              {i > 0 && <br />}
              <code>{line}</code>
            </>
          ))}
          {hasMessageOp && (
            <>
              <br />
              <span class="message-operation">
                {state.instructions.filter(inst => 
                  inst.includes('SEND') || inst.includes('RECEIVE') || inst.includes('DELIVER')
                ).map((inst, i) => (
                  <>
                    {i > 0 && ', '}
                    <i class="codicon codicon-arrow-right"></i> {inst}
                  </>
                ))}
              </span>
            </>
          )}
        </>
      );
    }
    
    return <code>{state.instructions.join('; ')}</code>;
  };

  const renderInteractiveChoices = () => {
    if (props.isFinished) {
      return (
        <div class="interactive-empty-state">
          <i class="codicon codicon-check-all empty-state-icon"></i>
          <h4>Execution Completed</h4>
          <p>The program has finished executing successfully.</p>
        </div>
      );
    }

    if (!props.interactiveStates || props.interactiveStates.length === 0) {
      return (
        <div class="interactive-empty-state">
          <i class="codicon codicon-debug-pause empty-state-icon"></i>
          <h4>No Choices Available</h4>
          <p>No more execution paths to choose from at this step.</p>
        </div>
      );
    }

    return (
      <div class="interactive-choices-container">
        <div class="choices-header">
          <h4>Choose Next Instruction</h4>
          <span class="choices-count">{props.interactiveStates.length} option{props.interactiveStates.length !== 1 ? 's' : ''}</span>
        </div>
        <div class="interactive-choices-list">
          {props.interactiveStates.map((state: NextStateOption, index: number) => (
            <button 
              class="interactive-choice-item"
              onClick={() => props.onExecuteStep(index)}
            >
              <div class="choice-header">
                <span class="choice-program">{state.prog_name}</span>
                <span class="choice-id">#{state.prog_id}</span>
              </div>
              <div class="choice-instruction">
                {formatInstructions(state)}
              </div>
              <i class="codicon codicon-chevron-right choice-arrow"></i>
            </button>
          ))}
        </div>
      </div>
    );
  };

  const renderVMState = () => {
    if (!props.currentVMState) {
      return (
        <div class="interactive-empty-state">
          <i class="codicon codicon-search empty-state-icon"></i>
          <h4>No State Information</h4>
          <p>VM state will appear here during execution.</p>
        </div>
      );
    }

    const state = props.currentVMState;
    
    return (
      <div class="vm-state-container" style="height: 100%; display: flex; flex-direction: column;">
        <VMStateInspector node={state} onClose={() => {}} />
        <div class="interactive-empty-state" style="flex: 1; justify-content: flex-start; padding-top: 20px;">
           <p style="font-size: 11px; opacity: 0.7;">Interactive state view. Use the choices tab to progress.</p>
        </div>
      </div>
    );
  };

  const renderRightPanelContent = () => {
    if (rightPanelTab() === "execution") {
      return (
        <div class="console" ref={executionConsoleRef}>
          {props.executionError ? (
            <div class="execution-error-box">
              <pre>{props.executionOutput}</pre>
            </div>
          ) : (
            <pre>{props.executionOutput}</pre>
          )}
        </div>
      );
    } else if (props.isRun !== false) { // When running (not in check mode)
      if (rightPanelTab() === "console") {
        return (
          <div class="console">
            <pre>{props.stdout || "The console output will appear here.\n\nAs you execute steps, any print statements or program output will be shown here in real-time."}</pre>
          </div>
        );
      } else if (rightPanelTab() === "msg_flow") {
        return (
          <div class="console">
            {renderMessageFlowGraph(props.interactiveMessageFlow || [], props.interactiveVmStates || [], props.editor)}
          </div>
        );
      } else if (rightPanelTab() === "vm_states") {
        return (
          <div class="console">
            {rendervmStates(props.interactiveVmStates || [], props.editor, props.interactiveStepLines)}
          </div>
        );
      }
    } else {
      // In check mode - show graph (placeholder for now)
      return (
        <div class="console">
          <div class="interactive-empty-state">
            <i class="codicon codicon-graph empty-state-icon"></i>
            <h4>Graph View</h4>
            <p>Graph visualization would appear here in check mode.</p>
          </div>
        </div>
      );
    }
    return null;
  };  
  
  return (
    <Show when={props.isVisible}>
      <div class={`interactive-panel ${isMinimized() ? 'minimized' : ''}`}>
        <div class="panel-header">
          <div class="panel-title">
            <i class="codicon codicon-debug-step-over panel-icon"></i>
            <h3>Interactive Run</h3>
          </div>
          
          <div class="panel-controls">
            <button
              class="control-button reset-button"
              onClick={() => props.onReset()}
              title="Reset Interactive Session"
            >
              <i class="codicon codicon-refresh"></i>
              Restart
            </button>
            <button 
              class="control-button minimize-button"
              onClick={() => toggleMinimize()}
              title={isMinimized() ? "Expand Panel" : "Minimize Panel"}
            >
              <i class={`codicon ${isMinimized() ? 'codicon-chevron-up' : 'codicon-chevron-down'}`}></i>
              {isMinimized() ? "Expand" : "Minimize"}
            </button>
            <button 
              class="control-button close-button"
              onClick={props.onClose}
              title="Close Interactive Mode"
            >
              <i class="codicon codicon-close"></i>
            </button>
          </div>
        </div>

        <Show when={!isMinimized()}>
          <div class="interactive-main-content">
            <Resizable id="interactive-content">
              {/* Left Panel - Interactive Choices */}
              <Resizable.Panel 
                class="interactive-left-panel"
                initialSize={0.68}
                minSize={0.3}
              >
                <div class="panel-tabs">
                  <button 
                    class={`tab-button ${activeTab() === 'choices' ? 'active' : ''}`}
                    onClick={() => setActiveTab('choices')}
                  >
                    <i class="codicon codicon-list-ordered"></i> Instructions
                  </button>
                  <button 
                    class={`tab-button ${activeTab() === 'state' ? 'active' : ''}`}
                    onClick={() => setActiveTab('state')}
                  >
                    <i class="codicon codicon-debug-console"></i> Current State
                  </button>
                </div>
                
                <div class="panel-content">
                  <div class="panel-body">
                    <Show when={activeTab() === 'choices'} fallback={renderVMState()}>
                      {renderInteractiveChoices()}
                    </Show>
                  </div>
                </div>
              </Resizable.Panel>

              <Resizable.Handle class="Resizable-handle"/>

              {/* Right Panel - Execution Details */}
              <Resizable.Panel 
                class="interactive-right-panel"
                initialSize={0.32}
                minSize={0.2}
              >
                <div class="right-panel-tabs">
                  <button 
                    class={`right-tab-button ${rightPanelTab() === 'console' ? 'active' : ''}`}
                    onClick={() => handleExecutionTabClick('console')}
                    disabled={props.isRun === false}
                  >
                    <i class="codicon codicon-terminal"></i> Console
                  </button>
                  <button 
                    class={`right-tab-button ${rightPanelTab() === 'execution' ? 'active' : ''} ${props.executionError ? 'execution-error' : ''}`}
                    onClick={() => handleExecutionTabClick('execution')}
                  >
                    <i class="codicon codicon-play"></i> Execution
                  </button>
                  <button 
                    class={`right-tab-button ${rightPanelTab() === 'msg_flow' ? 'active' : ''}`}
                    onClick={() => handleExecutionTabClick('msg_flow')}
                    disabled={props.isRun === false}
                  >
                    <i class="codicon codicon-send"></i> Message flow
                  </button>
                  <button 
                    class={`right-tab-button ${rightPanelTab() === 'vm_states' ? 'active' : ''}`}
                    onClick={() => handleExecutionTabClick('vm_states')}
                  >
                    <i class="codicon codicon-type-hierarchy-sub"></i> VM states
                  </button>
                </div>
                
                <div class="right-panel-content">
                  {renderRightPanelContent()}
                </div>
              </Resizable.Panel>
            </Resizable>
          </div>
        </Show>
      </div>
    </Show>
  );
}
