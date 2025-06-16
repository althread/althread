/** @jsxImportSource solid-js */
import vis from "vis-network/dist/vis-network.esm";
import { createSignal, onCleanup, onMount } from "solid-js";
import {nodeToString} from "./Node";
import GraphToolbar from "./GraphToolbar";

export const rendervmStates = (vm_states) => {
    console.log(vm_states);
    let container: HTMLDivElement | undefined;
    let network: vis.Network | null = null;
    const [maximized, setMaximized] = createSignal(false);

    if (!vm_states || vm_states.length === 0) {
        return <pre>The VM states will appear here.</pre>;
    }

    onMount(() => {
        if (!container) {
            console.error("Graph container element not found.");
            return;
        }


        const nodes: any = [];
        const edges: any = [];

        vm_states.forEach((vm, i) =>{
            //one node for each vm state
            let vm_node = {id: `${i}`, label: nodeToString(vm), shape: "box", font: { align: "left" }};
            nodes.push(vm_node);
        })
        vm_states.forEach((vm,i) =>{
            //arrow between parent node and its child
            if (i < vm_states.length){
                edges.push({
                    from: i,
                    to: i+1,
                })
            }
        })
        const data = { nodes, edges };
        
        const options = {
            layout: {
                hierarchical: {
                    enabled: true,
                    direction: "LR", //idk why but it goes up-down with LR
                    nodeSpacing: 250,
                    sortMethod: "directed", 
                }
            },
            edges: {
                arrows: "to",
            },
            physics: true, 
        };
        
        network = new vis.Network(container, data, options);
        network.once('stabilized', function() {
            if (network) network.fit();
        });

        onCleanup(() => { if (network) network.destroy(); });
    });



    const handleMaximize = () => {
      setMaximized(!maximized());
    };

    const handleRecenter = () => {
      if (network) {
        network.fit();
      }
    };

    return (
      <div
        class={`state-graph${maximized() ? " maximized" : ""}`}
      >
        <div
          ref={container}
          style="width: 100%; height: 100%;"
        />
        <GraphToolbar
          onFullscreen={handleMaximize}
          onRecenter={handleRecenter}
          isFullscreen={maximized()}
        />
      </div>
    );
}
