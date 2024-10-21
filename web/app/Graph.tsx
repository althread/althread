import vis from "vis-network/dist/vis-network.esm";
import { createEffect, createMemo, onMount } from "solid-js"


export default (props) => {

    // DOM container
    var container;
    let nodesArray = [
    ];
    let nodes = new vis.DataSet(nodesArray);

    // create an array with edges
    let edgesArray = [
    ];
    let edges = new vis.DataSet(edgesArray);

    createEffect(() => {
        nodes.clear();
        edges.clear();
        nodes.add(props.nodes);
        edges.add(props.edges);
    });

    onMount(() => {

        // provide the data in the vis format
        var data = {
            nodes: nodes,
            edges: edges
        };
        var options = {
            layout: {
              hierarchical: {
                direction: "UD",
                sortMethod: "directed",
              },
            },
            edges: {
              arrows: "to",
            },
            physics: {
            enabled: true,
            /*barnesHut: {
                theta: 0.5,
                gravitationalConstant: -2000,
                centralGravity: 0.2,
                springLength: 150,
                springConstant: 0.04,
                damping: 0.4,
                avoidOverlap: 0.9
            },*/
              hierarchicalRepulsion: {
                avoidOverlap: 1,
              },
              /*repulsion: {
                springLength: 2000,
                nodeDistance: 500,
                centralGravity: 0.1,
              }*/
            },
          };

        // initialize your network!
        var network = new vis.Network(container, data, options);
    });
    return <div class="state-graph" ref={container} />

}