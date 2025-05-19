// @refresh granular
/** @jsxImportSource solid-js */
import { createSignal, onCleanup, onMount } from "solid-js";
import Resizable from '@corvu/resizable'
import { Example1 } from "./examples/example1";
import { Router, Route } from "@solidjs/router";

import init, { compile, run, check } from '../pkg/althread_web';
import createEditor from './Editor';
import Graph from "./Graph";
import { Logo } from "./assets/images/Logo";
import { EditorState } from "@codemirror/state";
import Home from "./Home"; // Assuming you have a Home component
import Tutorial from "./Tutorial"; // Import the Tutorial component


init().then(() => {
  console.log('loaded');
});


const literal = (value) => {
  if(Object.keys(value)[0] == "tuple") {
    return '('+(Object.values(value)[0] as any[]).map(literal).join(',')+')';
  }
  return value[Object.keys(value)[0]];//+'('+Object.values(value)[0]+')';
}

type Node = {
  channels: Map<any, any[]>,
  globals: Map<any, any>,
  locals: { [key: string]: [any[], any] }
};

const nodeToString = (n: Node) => {
  let label = 'channels:\n'+[
    ...Array.from(n.channels.entries()).map(
      ([k,v]) => k.join('.')+' <- '+(
        v.map(l => literal(l)).join(',')
        //&& Object.values(v)[0].map(l => literal(l)).join(',')
    )
    )
  ].join('\n');
  label += '\nGlobals: '+[...Array.from(n.globals.entries()).map(([k,v]) => k+'='+literal(v))].join(',');
  label += '\nLocals: \n'+Object.values(n.locals).map(l => 'pc:'+l[1]+' stack:['+l[0].map(v=>literal(v)).join(',')+']').join('\n');

  return label;
}


export default function App() {

  let defaultValue =  Example1;
  if(localStorage.getItem('source-code')) {
    defaultValue = localStorage.getItem('source-code')!;
  }


  let editor = createEditor({
    compile, 
    defaultValue,
    onValueChange: (value) => {localStorage.setItem('source-code', value);}
  });

  let [nodes, setNodes] = createSignal([]);
  let [edges, setEdges] = createSignal([]);
  let [isRun, setIsRun] = createSignal(true);

  let [stdout, setStdout] = createSignal("The console output will appear here.");
  let [out, setOut] = createSignal("The execution output will appear here.");
  return (
    <Router>
      <Route path="/tutorial" component={Tutorial} />
      <Route path="/" component={() =>
        <>
          <div id="header">
              <div class="brand">
                <Logo />
                <h3>Althread</h3>
              </div>
              <div class="actions">
                <button
                  class="vscode-button" 
                  onClick={() => {
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
                  Load Example</button>
                <button 
                  class="vscode-button"
                  onClick={(e) => {
                    try {
                      setIsRun(true);
                      let res = run(editor.editorView().state.doc.toString());
                      setOut(res.debug);
                      setStdout(res.stdout.join('\n'));
                    } catch(e) {
                      setOut("ERROR: "+(e.pos && ('line '+e.pos.line))+"\n"+e.message);
                    }
                  }}>
                    <i class="codicon codicon-play"></i>
                    Run</button>
                  <button 
                    class="vscode-button"
                    onClick={(e) => {
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
                      });
                      setEdges(edges);
                      setIsRun(false);

                    } catch(e) {
                      setOut("ERROR: "+(e.pos && ('line '+e.pos.line))+"\n"+e.message);
                    }
                  }}>
                    <i class="codicon codicon-check"></i>
                    Check</button>
                    <button 
                  class="vscode-button"
                  onClick={(e) => {
                    setIsRun(true);
                    setOut("The execution output will appear here.");
                    setStdout("The console output will appear here.");
                    setNodes([]);
                    setEdges([]);
                  }}>
                    <i class="codicon codicon-clear-all"></i>
                    Reset</button>
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

    
              <div>
                <h3>Console</h3>
                <div class="console">
                  <pre>{stdout()}</pre>
                </div>
              </div>
              <div>
                <h3>Execution</h3>
                {isRun() ? (
                <div class="console">
                    <div>
                      <pre>{out()}</pre>
                    </div>
                </div>
              ) : (
                <Graph nodes={nodes()} edges={edges()} />
              )}
            </div>
              </Resizable.Panel>
          </Resizable>
        </>
      } />
    </Router>
  );
}

