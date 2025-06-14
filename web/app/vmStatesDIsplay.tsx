/** @jsxImportSource solid-js */
import {For} from "solid-js";
import vis from "vis-network/dist/vis-network.esm";
import { createEffect, onCleanup, onMount } from "solid-js";
import {nodeToString} from "./App.tsx";

export const rendervmStates = (vm_states) => {
    console.log(vm_states);
    let container!: HTMLDivElement;

    if (!vm_states || vm_states.length === 0) {
        return <pre>The VM states will appear here.</pre>;
    }

    onMount(() => {
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
        
        var network = new vis.Network(container, data, options);
        network.once('stabilized', function() {
            network.fit();
        });

        onCleanup(() => network.destroy());





    });
    return <div class="state-graph" ref={container} />;
}
