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
import { extractProgs } from "./vmUtils";
import { rendervmStates } from "./vmStatesDisplay";
import { nodeToString, node_entirely } from "./Node";



init().then(() => {
  console.log('loaded');
});


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

  let [activetab, setActivetab] = createSignal("console");
  const handleTabClick = (tab: string) => {
    setActivetab(tab);
  };

  let [nodes, setNodes] = createSignal([]);
  let [edges, setEdges] = createSignal([]);
  let [isRun, setIsRun] = createSignal(true);

  let [stdout, setStdout] = createSignal("The console output will appear here.");
  let [out, setOut] = createSignal("The execution output will appear here.");
  let [commgraphout, setCommGraphOut] = createSignal([]); //messageflow graph
  let [prog_list, setProgList] = createSignal<any[]>([]); //for the messageflow graph
  let [vm_states, setVmStates] = createSignal<any[]>([]); //to display vm states information
  let [activeAction, setActiveAction] = createSignal<string | null>(null);


  const renderExecContent = () => {
    if (isRun()) {
      if (activetab() === "console") {
        return (
          <div class="console">
            <pre>{stdout()}</pre>
          </div>
        );
      } else if (activetab() === "execution") {
        return (
          <div class="console">
            <pre>{out()}</pre>
          </div>
        );
      } else if (activetab() === "msg_flow") {
        return (
          <div class="console">
            {renderMessageFlowGraph(commgraphout(), prog_list(), vm_states())}
          </div>
        );
      } else if (activetab() === "vm_states") {
        return (
          <div class="console">
            {rendervmStates(vm_states())}
          </div>
        );
      }
    } else {
      setActivetab("vm_states");
      return (
        <div class="console">
          <Graph nodes={nodes()} edges={edges()} />
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
              class={`vscode-button${activeAction() === "load" ? " active" : ""}`}
              onClick={() => {
                setActiveAction(activeAction() === "load" ? null : "load");
                let up = editor.editorView().state.update({
                  changes: {
                    from: 0, 
                    to: editor.editorView().state.doc.length,
                    insert: Example1
                  }
                })
                editor.editorView().update([up]);
              }
              }>
              <i class="codicon codicon-file"></i>
              Load Example
            </button>

            <button
              class={`vscode-button${activeAction() === "run" ? " active" : ""}`}
              onClick={() => {
                setActiveAction(activeAction() === "run" ? null : "run");
                try {
                  setIsRun(true);
                  let res = run(editor.editorView().state.doc.toString());
                  let proglist = extractProgs(res.vm_states);
                  setProgList(proglist);
                  console.log(res.vm_states);
                  setOut(res.debug);
                  setCommGraphOut(res.message_flow_graph); //set the message flow data
                  setVmStates(res.vm_states);
                  setStdout(res.stdout.join('\n'));
                } catch(e: any) {
                  setOut("ERROR: "+(e.pos && ('line '+e.pos.line))+"\n"+e.message);
                }
              }}>
              <i class="codicon codicon-play"></i>
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
                    return {
                      id: i,
                      level,
                      label,
                      color: colored_path.includes(label) || (colored_path.length>0 && level == 0)  ? "#ec9999" : "#a6dfa6",
                      shape: "box",
                      font: { align: "left" },
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
                    console.log(node_entirely(n[0]));
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
              class={`vscode-button${activeAction() === "reset" ? " active" : ""}`}
              onClick={() => {
                setActiveAction(activeAction() === "reset" ? null : "reset");
                setIsRun(true);
                setOut("The execution output will appear here.");
                setStdout("The console output will appear here.");
                setCommGraphOut([]);
                setNodes([]);
                setEdges([]);
                setVmStates([]);
              }}>
              <i class="codicon codicon-clear-all"></i>
              Reset
            </button>

            <button
              class={`vscode-button${activeAction() === "tutorial" ? " active" : ""}`}
              onClick={() => {
                setActiveAction(activeAction() === "tutorial" ? null : "tutorial");
                navigate('/tutorial');
              }}>
              <i class="codicon codicon-book"></i>
              Tutorials
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
                <button class={`tab_button ${activetab() === "console" ? "active" : ""}`}
                        onclick={() => handleTabClick("console")}
                        disabled={!isRun()}
                >
                  <h3>Console</h3>
                </button>
                <button class={`tab_button ${activetab() === "execution" ? "active" : ""}`}
                        onclick={() => handleTabClick("execution")}
                        disabled={!isRun()}
                >
                  <h3>Execution</h3>
                </button>
                <button class={`tab_button ${activetab() === "msg_flow" ? "active" : ""}`}
                        onclick={() => handleTabClick("msg_flow")}
                        disabled={!isRun()}
                >
                  <h3>Message flow</h3>
                </button>
                <button class={`tab_button ${activetab() === "vm_states" ? "active" : ""}`}
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

