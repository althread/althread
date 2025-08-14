/** @jsxImportSource solid-js */
import { createEffect, createSignal, Show } from "solid-js";
import Resizable from '@corvu/resizable'
import { renderMessageFlowGraph } from "@components/graph/CommGraph";
import { rendervmStates } from "@components/graph/vmStatesDisplay";
import './InteractivePanel.css';

interface InteractivePanelProps {
  isVisible: boolean;
  interactiveStates: any[];
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
          {props.interactiveStates.map((state: any, index: number) => (
            <button 
              class="interactive-choice-item"
              onClick={() => props.onExecuteStep(index)}
            >
              <div class="choice-header">
                <span class="choice-program">{state.get('prog_name')}</span>
                <span class="choice-id">#{state.get('prog_id')}</span>
              </div>
              <div class="choice-instruction">
                {state.get('instruction_preview')}
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
    console.log("Current VM State:", state);
    console.log("Current VM State keys:", Array.from(state.keys()));
    console.log("Channel connections:", state.get('channel_connections'));
    
    return (
      <div class="vm-state-container">
        <div class="state-section">
          <h5><i class="codicon codicon-symbol-variable"></i> Global Variables</h5>
          <div class="state-content">
            {state.get('globals') && Array.from(state.get('globals')).length > 0 ? (
              <div class="globals-list">
                {Array.from(state.get('globals')).map((entry: any) => (
                  <div class="global-item">
                    <span class="global-name">{entry[0]}:</span>
                    <span class="global-value">{entry[1]}</span>
                  </div>
                ))}
              </div>
            ) : (
              <pre>No global variables.</pre>
            )}
          </div>
        </div>
        
        <div class="state-section">
          <h5><i class="codicon codicon-arrow-swap"></i> Channels</h5>
          <div class="state-content">
            <div class="channel-subsection">
              <h6>Channel States:</h6>
              {state.get('channels') && Array.from(state.get('channels')).length > 0 ? (
                <div class="channels-list">
                  {Array.from(state.get('channels')).map((channel: any) => (
                    <div class="channel-item">
                      <div class="channel-header">
                        <strong>Channel: {channel.get('name')} (PID: {channel.get('pid')})</strong>
                      </div>
                      <div class="channel-values">
                        <div class="channel-values-label">Messages:</div>
                        {channel.get('values') && channel.get('values').length > 0 ? (
                          <div class="channel-messages">
                            {channel.get('values').map((message: string) => {
                              // Parse the message format: "Tuple([Tuple([Int(0), Int(1)]), Tuple([Int(125), Bool(true)])])"
                              // First tuple: [sender_pid, receiver_pid], Second tuple: message content
                              const match = message.match(/Tuple\(\[Tuple\(\[Int\((\d+)\), Int\((\d+)\)\]\), (.+)\]\)/);
                              if (match) {
                                const senderPID = match[1];
                                const messageContent = match[3];
                                return (
                                  <div class="message-item">
                                    <div class="message-header">
                                      <span class="message-sender">From PID {senderPID}</span>
                                    </div>
                                    <div class="message-content">{messageContent}</div>
                                  </div>
                                );
                              }
                              return (
                                <div class="message-item">
                                  <div class="message-content">{message}</div>
                                </div>
                              );
                            })}
                          </div>
                        ) : (
                          <div class="no-messages">No messages</div>
                        )}
                      </div>
                    </div>
                  ))}
                </div>
              ) : (
                <pre>No messages sent/received.</pre>
              )}
            </div>
            <Show when={state.get('channel_connections') && Array.from(state.get('channel_connections')).length > 0}>
              <div class="channel-subsection">
                <h6>Channel Connections:</h6>
                <div class="connections-list">
                  {Array.from(state.get('channel_connections')).map((connection: any) => (
                    <div class="connection-item">
                      <div class="connection-line">
                        <span class="connection-source">
                          PID {connection.get('from').get('pid')} ({connection.get('from').get('channel')})
                        </span>
                        <span class="connection-arrow">→</span>
                        <span class="connection-target">
                          PID {connection.get('to').get('pid')} ({connection.get('to').get('channel')})
                        </span>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </Show>
          </div>
        </div>
        
        <div class="state-section">
          <h5><i class="codicon codicon-gear"></i> Programs</h5>
          <div class="state-content">
            {state.get('programs').map((prog: any) => (
              <div class="program-info">
                <div class="program-header">
                  <strong class="text-with-icon"><i class="codicon codicon-symbol-function"></i> {prog.get('name')}</strong>
                  <span class="program-pid">PID: {prog.get('pid')}</span>
                </div>
                <div class="program-details">
                  <div class="text-with-icon"><i class="codicon codicon-debug-step-over"></i> IP: {prog.get('instruction_pointer')}</div>
                  <div class="memory-section">
                    <div class="memory-label"><i class="codicon codicon-symbol-array"></i> Memory:</div>
                    {prog.get('memory') && prog.get('memory').length > 0 ? (
                      <div class="memory-list">
                        {prog.get('memory').map((item: string, index: number) => (
                          <div class="memory-item">
                            <span class="memory-index">[{index}]</span>
                            <span class="memory-value">{item}</span>
                          </div>
                        ))}
                      </div>
                    ) : (
                      <div class="memory-empty">Empty stack</div>
                    )}
                  </div>
                </div>
              </div>
            ))}
          </div>
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
            {renderMessageFlowGraph(props.interactiveMessageFlow || [], props.interactiveVmStates || [])}
          </div>
        );
      } else if (rightPanelTab() === "vm_states") {
        return (
          <div class="console">
            {rendervmStates(props.interactiveVmStates || [])}
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
  };  return (
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
                    <i class="codicon codicon-debug-console"></i> Quick State
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
