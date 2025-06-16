/** @jsxImportSource solid-js */
import vis from "vis-network/dist/vis-network.esm";
import { createEffect, onCleanup, onMount, createSignal } from "solid-js";
import GraphToolbar from "./GraphToolbar";

export default (props /*: GraphProps*/) => {
    let container: HTMLDivElement | undefined; // Renamed for clarity
    let network: vis.Network | null = null;
    const [maximized, setMaximized] = createSignal(false);

    const nodes = new vis.DataSet(props.nodes || []);
    const edges = new vis.DataSet(props.edges || []);

    createEffect(() => {
        nodes.clear();
        nodes.add(props.nodes || []);
        edges.clear();
        edges.add(props.edges || []);
    });

    onMount(() => {
        if (!container) {
            console.error("Graph container element not found.");
            return;
        }

        const data = {
            nodes: nodes.get(),
            edges: edges.get()
        };

        const options = {
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
                hierarchicalRepulsion: {
                    avoidOverlap: 1,
                },
            },
        };

        network = new vis.Network(container, data, options);

        onCleanup(() => {
            if (network) {
                network.destroy();
                network = null;
            }
        });
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
};