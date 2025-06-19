// @refresh granular
/** @jsxImportSource solid-js */
import { createSignal } from "solid-js";
import Resizable from '@corvu/resizable'
import { Example1 } from "./examples/example1";
import { useNavigate } from "@solidjs/router";

import init, { compile, run, check } from '../pkg/althread_web';
import createEditor from './Editor';
import Graph from "./Graph";
import { Logo } from "./assets/images/Logo";
import {renderMessageFlowGraph} from "./CommGraph";
import { rendervmStates } from "./vmStatesDisplay";
import { nodeToString, node_entirely } from "./Node";



init().then(() => {
  console.log('loaded');
});

const animationTimeOut = 100; //ms

export default function App() {

  const navigate = useNavigate();

  let defaultValue =  Example1;
  if(localStorage.getItem('source-code')) {
    defaultValue = localStorage.getItem('source-code')!;
  }

  let editor = createEditor({
    compile, 
    defaultValue,
    onValueChange: (value) => {localStorage.setItem('source-code', value);}
  });

  let [activeTab, setActiveTab] = createSignal("console");
  const handleTabClick = (tab: string) => {
    setActiveTab(tab);
  };

  let [nodes, setNodes] = createSignal([]);
  let [edges, setEdges] = createSignal([]);
  let [isRun, setIsRun] = createSignal(true);

  let [stdout, setStdout] = createSignal("The console output will appear here.");
  let [out, setOut] = createSignal("The execution output will appear here.");
  let [commgraphout, setCommGraphOut] = createSignal([]); //messageflow graph
  let [vm_states, setVmStates] = createSignal<any[]>([]); //to display vm states information
  let [activeAction, setActiveAction] = createSignal<string | null>(null);
  const [loadingAction, setLoadingAction] = createSignal<string | null>(null);


  const renderExecContent = () => {
    if (isRun()) {
      if (activeTab() === "console") {
        return (
          <div class="console">
            <pre>{stdout()}</pre>
          </div>
        );
      } else if (activeTab() === "execution") {
        return (
          <div class="console">
            <pre>{out()}</pre>
          </div>
        );
      } else if (activeTab() === "msg_flow") {
        return (
          <div class="console">
            {renderMessageFlowGraph(commgraphout(), vm_states())}
          </div>
        );
      } else if (activeTab() === "vm_states") {
        return (
          <div class="console">
            {rendervmStates(vm_states())}
          </div>
        );
      }
    } else {
      setActiveTab("vm_states");
      return (
        <div class="console">
          <Graph nodes={nodes()} edges={edges()} theme="dark" />
        </div>
      );
    }
  };


  return (
    <>
      <div id="header">
          <div class="brand">
            <Logo />
            <h3>Althread</h3>
          </div>
          <div class="actions">
            <button
              class={`vscode-button${loadingAction() === "load" ? " active" : ""}`}
              onClick={async () => {
                setLoadingAction("load");
                try {
                let up = editor.editorView().state.update({
                  changes: {
                    from: 0, 
                    to: editor.editorView().state.doc.length,
                    insert: Example1
                  }
                })
                editor.editorView().update([up]);
              } catch (error) {
                console.error("Error loading example:", error);
              } finally {
                setTimeout(() => {
                    setLoadingAction(null);
                    setActiveAction(null);
                }, animationTimeOut);
              }
              }}>
              <i class={loadingAction() === "load" ? "codicon codicon-loading codicon-modifier-spin" : "codicon codicon-file"}></i>
              Load Example
            </button>

            <button
              class={`vscode-button${loadingAction() === "run" ? " active" : ""}`}
              disabled={loadingAction() === "run"}
              onClick={async () => {
                setLoadingAction("run");
                try {
                  setIsRun(true);
                  let res = run(editor.editorView().state.doc.toString());
                  setOut(res.debug);
                  setCommGraphOut(res.message_flow_graph);
                  setVmStates(res.vm_states);
                  setStdout(res.stdout.join('\n'));
                  setActiveTab("console");
                } catch(e: any) {
                  setOut("ERROR: "+(e.pos && ('line '+e.pos.line))+"\n"+e.message);
                } finally {
                  setTimeout(() => {
                    setLoadingAction(null);
                    setActiveAction(null);
                  }, animationTimeOut);
                }
              }}>
              <i class={loadingAction() === "run" ? "codicon codicon-loading codicon-modifier-spin" : "codicon codicon-play"}></i>
              Run
            </button>

            <button
              class={`vscode-button${activeAction() === "check" ? " active" : ""}`}
              onClick={() => {
                setActiveAction(activeAction() === "check" ? null : "check");
                try {
                  let res = check(editor.editorView().state.doc.toString())
                  setOut(res);
                  
                  console.log(res);
                  let colored_path: string[] = [];
                  if(res[0].length > 0) { // a violation occurred
                    res[0].forEach((path) => {
                      colored_path.push(nodeToString(path.to));
                    });
                  }

                  let nodes = {};
                  setNodes(res[1].nodes.map((n, i) => {
                    let label = nodeToString(n[0]);
                    const {level, predecessor, successors} = n[1];
                    nodes[label] = i;
                    const isViolationNode = colored_path.includes(label) || (colored_path.length > 0 && level == 0);
                    return {
                      id: i,
                      level,
                      label,
                      color: {
                        border: isViolationNode ? "#ec9999" : "#a6dfa6",
                        background: isViolationNode ? "#4d3131" : "#314d31"
                      }
                    }
                  }));

                  let edges: any = [];
                  res[1].nodes.forEach((n, i) => {
                    let label = nodeToString(n[0]);
                    const {level, predecessor, successors} = n[1];
                    successors.forEach(({lines, pid, name, to}) => {
                      to = nodeToString(to);
                      edges.push({
                        from: i,
                        to: nodes[to],
                        label: name+'#'+pid+': '+lines.join(',')
                      });
                    })
                    // console.log(node_entirely(n[0]));
                  });
                  setEdges(edges);
                  setIsRun(false);

                } catch(e: any) {
                  setOut("ERROR: "+(e.pos && ('line '+e.pos.line))+"\n"+e.message);
                }
              }}>
              <i class="codicon codicon-check"></i>
              Check
            </button>

            <button
              class={`vscode-button${loadingAction() === "reset" ? " active" : ""}`}
              onClick={async () => {
                setLoadingAction("reset");
                try {
                  setIsRun(true);
                  setOut("The execution output will appear here.");
                  setStdout("The console output will appear here.");
                  setCommGraphOut([]);
                  setNodes([]);
                  setEdges([]);
                  setVmStates([]);
                } finally {
                  setTimeout(() => {
                    setLoadingAction(null);
                  }, 100);
                }
              }}>
              <i class={loadingAction() === "reset" ? "codicon codicon-loading codicon-modifier-spin" : "codicon codicon-clear-all"}></i>
              Reset
            </button>

            <button
              class={`vscode-button${loadingAction() === "tutorial" ? " active" : ""}`}
              onClick={() => {
                setLoadingAction("tutorial");
                navigate('/tutorials');
              }}>
              <i class="codicon codicon-book"></i>
              Tutorials
            </button>
            <button
              class={`vscode-button${loadingAction() === "docs" ? " active" : ""}`}
              onClick={() => {
                setLoadingAction("docs");
                navigate('/docs/guide/intro');
              }}>
              <i class="codicon codicon-repo"></i>
              Documentation
            </button>
          </div>
      </div>
      <Resizable id="content">
        <Resizable.Panel class="editor-panel"
          initialSize={0.55}
          minSize={0.2}>
          <div ref={editor.ref} />
        </Resizable.Panel>
        <Resizable.Handle class="Resizable-handle"/>
        <Resizable.Panel class="right-panel"
          initialSize={0.45}
          minSize={0.2}>

            <div class="execution-content">
              <div class="tab">
                <button class={`tab_button ${activeTab() === "console" ? "active" : ""}`}
                        onclick={() => handleTabClick("console")}
                        disabled={!isRun()}
                >
                  <h3>Console</h3>
                </button>
                <button class={`tab_button ${activeTab() === "execution" ? "active" : ""}`}
                        onclick={() => handleTabClick("execution")}
                        disabled={!isRun()}
                >
                  <h3>Execution</h3>
                </button>
                <button class={`tab_button ${activeTab() === "msg_flow" ? "active" : ""}`}
                        onclick={() => handleTabClick("msg_flow")}
                        disabled={!isRun()}
                >
                  <h3>Message flow</h3>
                </button>
                <button class={`tab_button ${activeTab() === "vm_states" ? "active" : ""}`}
                        onclick={() => handleTabClick("vm_states")}
                >
                  <h3>VM states</h3>
                </button>
              </div>

              <div class="tab-content">
                {renderExecContent()}
              </div>
            </div>

</Resizable.Panel>
      </Resizable>
    </>
  );
}

