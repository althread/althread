// @refresh granular

import { createSignal, onCleanup, onMount } from "solid-js";
import Resizable from '@corvu/resizable'

import init, { compile, run, check } from '../pkg/althread_web';
import createEditor from './Editor';
import Graph from "./Graph";


init().then(() => {
  console.log('loaded');
});


const literal = (value) => {
  if(Object.keys(value)[0] == "tuple") {
    return '('+Object.values(value)[0].map(literal).join(',')+')';
  }
  return value[Object.keys(value)[0]];//+'('+Object.values(value)[0]+')';
}

const nodeToString = (n) => {
  let label = 'channels:\n'+[
    ...n.channels.entries().map(
      ([k,v]) => k.join('.')+' <- '+(
        v.map(l => literal(l)).join(',')
        //&& Object.values(v)[0].map(l => literal(l)).join(',')
    )
    )
  ].join('\n');
  label += '\nGlobals: '+[...n.globals.entries().map(([k,v]) => k+'='+literal(v))].join(',');
  label += '\nLocals: \n'+Object.values(n.locals).map(l => 'pc:'+l[1]+' stack:['+l[0].map(v=>literal(v)).join(',')+']').join('\n');

  return label;
}


export default function App() {
  let editor = createEditor(compile);

  let [nodes, setNodes] = createSignal([]);
  let [edges, setEdges] = createSignal([]);

  let [out, setOut] = createSignal("Output will appear here");
  return (
    <>
      <div id="header">Althread Editor</div>
      <Resizable id="content">
        <Resizable.Panel class="editor-panel"
          initialSize={0.6}
          minSize={0.2}>
          <div ref={editor.ref} />
        </Resizable.Panel>
        <Resizable.Handle />
        <Resizable.Panel class="right-panel"
          initialSize={0.4}
          minSize={0.2}>
          <button onClick={(e) => {
            try {
              let res = run(editor.editorView().state.doc.toString());
              console.log('result:', res);
              setOut(res.debug);
            } catch(e) {
              setOut("ERROR: "+(e.pos && ('line '+e.pos.line))+"\n"+e.message);
            }
          }}>Run</button>

          <button onClick={(e) => {
            try {
              let res = check(editor.editorView().state.doc.toString())
              setOut(res);
              
              console.log(res);
              let colored_path = [];
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

            } catch(e) {
              setOut("ERROR: "+(e.pos && ('line '+e.pos.line))+"\n"+e.message);
            }
          }}>Check</button>
          <pre>
          {out()}
          </pre>
          <Graph nodes={nodes()} edges={edges()} />
        </Resizable.Panel>
      </Resizable>
    </>
  );
}

