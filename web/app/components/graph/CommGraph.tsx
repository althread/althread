/** @jsxImportSource solid-js */
import vis from "vis-network/dist/vis-network.esm";
import { createSignal, onCleanup, createEffect, Show } from "solid-js"
import { literal, Node} from "./Node";
import { ProgramStateJS } from "./State";
import GraphToolbar from "./GraphToolbar";
import { exportCommGraphToCSV } from "./exportToCSV";

import { createGraphToolbarHandlers } from "./visHelpers";
import { useGraphMaximizeHotkeys } from "@hooks/useGraphMaximizeHotkeys";

interface MessageFlowEvent {
    sender: number, // id of the sending process
    receiver: number,  // id of the receiving process
    evt_type: number, //send or receive
    message: string, // for SEND: channel name, for RECV: message content
    number: number, // message sequence number (nmsg_sent for SEND, clock for RECV)
    actor_prog_name: string, // Name of the program performing this action
    vm_state: any
};

interface MessageNode {
  id: string;
  y: number;
  x: number;
  shape: string;
  size: number;
  event: MessageFlowEvent | null; 
  broadcast: boolean | null;
  color: string;
}

const VmStatePopup = (props: { event: MessageFlowEvent }) => {
  const [activeTab, setActiveTab] = createSignal('details');
  const vmState: Node = props.event.vm_state;

  if (!vmState) {
    return <div>VM state is not available.</div>;
  }

  const eventType = String.fromCharCode(props.event.evt_type);

  return (
    <div class="vm-state-popup-content">
      <div class="popup-tabs">
        <button
          class={`popup-tab-button ${activeTab() === 'details' ? 'active' : ''}`}
          onClick={() => setActiveTab('details')}
        >
          Event Details
        </button>
        <button
          class={`popup-tab-button ${activeTab() === 'vmState' ? 'active' : ''}`}
          onClick={() => setActiveTab('vmState')}
        >
          VM State
        </button>
      </div>
      <div class="popup-tab-content">
        <Show when={activeTab() === 'details'}>
          <p><strong>Type:</strong> {eventType === 's' ? 'Send' : 'Receive'}</p>
          <p><strong>Sender:</strong> {props.event.sender}</p>
          <p><strong>Receiver:</strong> {props.event.receiver ?? 'Broadcast'}</p>
          <p><strong>Message:</strong> {props.event.message}</p>
        </Show>
        <Show when={activeTab() === 'vmState'}>
          <div>
            <strong>Globals:</strong>
            {vmState.globals && vmState.globals.size > 0 ? (
              <ul>
                {[...Array.from(vmState.globals.entries()).map(
                  ([k, v]) => <li>{String(k)} = {literal(v)}</li>
                )]}
              </ul>
            ) : (
              <p style={{"padding-left": "1rem", "font-style": "italic"}}>No global variables.</p>
            )}
          </div>
          <div>
            <strong>Program States & Channels:</strong>
            {vmState.locals && vmState.locals.length > 0 ? (
              vmState.locals.map((prog_state: ProgramStateJS) => (
                <div class="program-state">
                  <p>
                    <strong>Program {prog_state.name}</strong> (pid {prog_state.pid}, clock {prog_state.clock})
                  </p>
                  <ul>
                    <li><strong>pc:</strong> {prog_state.instruction_pointer}</li>
                    <li><strong>stack:</strong> [{prog_state.memory.map(v => literal(v)).join(', ')}]</li>
                    <li>
                      <strong>Channels:</strong>
                      {(() => {
                        const programChannels: any[] = [];
                        if (vmState.channels && vmState.channels.size > 0) {
                          for (const [key, value] of vmState.channels.entries()) {
                            if (Array.isArray(key) && key.length === 2 && key[0] === prog_state.pid) {
                              const channelName = key[1];
                              programChannels.push(
                                <li>{channelName} &lt;- {Array.isArray(value) ? value.map(l => literal(l)).join(',') : String(value)}</li>
                              );
                            }
                          }
                        }
                        return programChannels.length > 0 ? <ul>{programChannels}</ul> : <p style={{"padding-left": "1rem", "font-style": "italic"}}>No active input channels.</p>;
                      })()}
                    </li>
                  </ul>
                </div>
              ))
            ) : (
              <p style={{"padding-left": "1rem", "font-style": "italic"}}>No running programs</p>
            )}
          </div>
        </Show>
      </div>
    </div>
  );
};


export const printCommGrapEventList = (eventl: any) => {
  //debug purpose
  try{
    if (eventl.length === 0) {
      return (<pre>No MessageFlow events recorded.</pre>);
    }
    else return (
      <>
        {eventl && eventl.length > 0 ? (
          <ul>
            {eventl.map((event, index) => (
              <li>
                {event.sender} → {event.receiver == null ? "Broadcast" : event.receiver}: {event.message}
              </li>
            ))}
          </ul>
        ) : (
          <p>No MessageFlow events recorded.</p>
        )}
      </>
    );
  }catch(e){
    console.log(e.message);
    return(<div class="console">erreur d'affichage : {e.message}</div>);
  }
}

const matchedSendNodeIds = new Set<string>();

const searchSenderNode = (size, graphNodes, receivingEvent, msgNum, broadcast) => {
  //searches for the sending event corresponding to the receiving event
  let suite = "";
  (broadcast) ? suite = "B" : suite = receivingEvent.receiver;
  let str = "p" + receivingEvent.sender + "_send" + "_to" + suite + "_" + msgNum;
  let sender_node = graphNodes.get(str);

  let number = size;
  graphNodes.forEach((node) => {
    if (node.event && node.event.evt_type === 115
        && node.event.sender === receivingEvent.sender
        && node.event.receiver === receivingEvent.receiver

        && !matchedSendNodeIds.has(node.id) // Check if the sender node has already been matched
    ) {
      if (number > node.event.number) {
        number = node.event.number; // Find the smallest number for the sender node
        sender_node = node;
      } 
    }
  });

  if (sender_node){
    matchedSendNodeIds.add(sender_node.id); // Mark this sender node as matched
  }
  return sender_node;
  
}



export const renderMessageFlowGraph = (commGraphData, vm_states) => {
  //returns div element to display the message flow graph and the vm states popup on click
  //prog_list = array of program names (strings)
  //commGraphData = array of communication events
  let container!: HTMLDivElement;
  let network: vis.Network | null = null;
  let popupRef: HTMLDivElement;
  const [maximized, setMaximized] = createSignal(false);
  let [popupVisible, setPopupVisible] = createSignal(false);
  let [popupContent, setPopupContent] = createSignal<MessageFlowEvent | null>(null);
  let [popupPosition, setPopupPosition] = createSignal({ x: 0, y: 0 });
  const [finalPopupPosition, setFinalPopupPosition] = createSignal({ x: 0, y: 0 });
  const [isPopupReady, setIsPopupReady] = createSignal(false);

  // Add this state to remember alignment
  const [popupAlignment, setPopupAlignment] = createSignal<{vertical: 'top'|'bottom', horizontal: 'left'|'right'}>({ vertical: 'bottom', horizontal: 'right' });

  if (!commGraphData || commGraphData.length === 0) {
    return (<pre>The communication graph will appear here (if any communication events are recorded).</pre>);
  }

  createEffect(() => {

    // Clear matchedSendNodeIds for each new graph
    matchedSendNodeIds.clear();

    const nodes= new vis.DataSet();
    const edges = new vis.DataSet();
    const processLines = new Map(); //coordinates of start&end of each process line
    const processes: Map<number, string> = new Map();

    // extract processes name to make one line per process
    commGraphData.forEach((event: MessageFlowEvent) => {
      if (event.message === "out") {
        processes.set(event.sender, event.actor_prog_name);
      } else {
        processes.set(event.receiver, event.actor_prog_name);
      }
    });

    let ySpacing = 150; // space between processes vertically
    let xStart = -50;
    let xEnd = 450;

    // lines generation
    processes.forEach((processName, index) => {

      // define two nodes for each process (start and end of the line)
      let startNode = { id: `p${index}_start`, y: index * ySpacing, x: xStart, shape: "dot", size: 1, color: "white" };
      let endNode = { id: `p${index}_end`, y: index * ySpacing, x: xEnd, shape: "dot", size: 1, color: "white" };
      nodes.add([startNode, endNode]);
      edges.add({ from: startNode.id, to: endNode.id, color: "white", width: 2 }); // line for each process
      
      let processNumberNode = {
        id: `p${index}_number`,
        y: index * ySpacing, 
        x: xStart - 40,
        label: `P${index} \n(${processName})`,
        shape: "text",
        size : 0,
        color: "white",
        font: { 
          size: 18,
          color: "white",
        },
      };
    
      nodes.add(processNumberNode); 

      processLines.set(index, { start: startNode.id, end: endNode.id });
    });

    // message arrows
    let i: number = 0;
    let broadcast: boolean = false;

    commGraphData.forEach((event: MessageFlowEvent) => {
      let yposLine = 0;
      let id_txt = "p";
      let evt_type = String.fromCharCode(event.evt_type);
            
      //lengthen the process lines for each new event so it doesnt go overboard
      processes.forEach((_, processNumber) => {
        nodes.update({id: `p${processNumber}_end`, x: 500 + commGraphData.length * 1.5 * i});
      });
      
      if (evt_type === 'r') { /// evt_type === 114
        yposLine = event.receiver; //reception -> node on receiver line
        id_txt += event.receiver.toString();
        id_txt += "_recv" + "_from" + event.sender + "_"  + event.number;
      }

      else {
        let id_suite;
        yposLine = event.sender;
        id_txt += event.sender.toString();
        
        if (event.receiver === undefined){
          broadcast = true;
          id_suite = "B";
        }
        else{
          broadcast = false; id_suite = event.receiver;
        }
        id_txt += "_send" + "_to" + id_suite + "_" + event.number;
      }

      let msgNode = { id: id_txt, y: yposLine * ySpacing, x: xStart+20+i*50, 
                      shape: "dot", size: 5, color: "#cccccc", event: event, broadcast: broadcast };
      nodes.add(msgNode);
      i++;
    });


    nodes.forEach((item, id) => {
      // item is a node object, id is its id
      let node: MessageNode = item as MessageNode;
      let evt_type = node.event?.evt_type !== undefined ? String.fromCharCode(node.event.evt_type) : "";
      
      if ((evt_type) === 'r'){
        let sender = searchSenderNode(commGraphData.length, nodes, node.event, node.event.number, node.broadcast);
        if (sender){
          edges.add({
            from: sender.id,
            to: node.id,
            label: node.event?.message,
            font:{
              size: 20,
              color: "white",
              align: "middle",
              background: "none",
              strokeWidth: 0,
            },
            arrows: "to",
            color: "hsla(29.329, 66.552%, 52.544%)", // theme orange
          })
        }
      }
    });

    const data = { nodes: nodes.get(), edges: edges.get() };

    const options = {
      layout: {
        hierarchical: false, 
      },
      edges: {
        smooth: false,
      },
      physics: false, 
      nodes:{
        fixed: true
      }
    };

    network = new vis.Network(container, data, options);
    network.once('stabilized', function() {
      if (network) network.fit();
    });

    /* to display the associated vm state when clicking on an event node */
    let previous_node_id: number | null = null;
    let previous_node_colour: string | null = null;
    network.on("click", (event) =>{
      if(previous_node_id){ //change previous clicked node back to its original colour
        nodes.update({id: previous_node_id, color: previous_node_colour});
        setPopupVisible(false);
      }
      if(event.nodes.length > 0){
        let node_id = event.nodes[0];
        //popup creation & change node colour
        if(!(node_id.includes("_start") || node_id.includes("_end")
                  || node_id.includes("_number"))){ //clicked node is only one of the communication event
          const node = nodes.get(node_id);
          let pos = event.pointer.DOM;
          setIsPopupReady(false);
          setPopupContent(node.event);
          setPopupPosition({x: pos.x, y: pos.y});
          setPopupAlignment({ vertical: 'bottom', horizontal: 'right' }); // <--- Reset alignment on new popup
          setPopupVisible(true);
          previous_node_id = node_id;
          previous_node_colour = node.color;
          nodes.update({id: node_id, color: "#0080ff"});
          
        }
      } else {
          setPopupVisible(false);
      }
    }
    );

    onCleanup(() => { if (network) network.destroy(); });
  });

  createEffect(() => {
    if (popupVisible() && popupRef) {
      const reposition = () => {
        if (!popupRef || !container) return;
        
        const graphContainer = container;
        const popupEl = popupRef;

        const graphWidth = graphContainer.offsetWidth;
        const graphHeight = graphContainer.offsetHeight;
        const popupWidth = popupEl.offsetWidth;
        const popupHeight = popupEl.offsetHeight;

        const initialPos = popupPosition();
        let finalX = initialPos.x + 15;
        let finalY = initialPos.y + 15;

        // Use current alignment as starting point
        let { vertical, horizontal } = popupAlignment();

        // Only change alignment if the popup would overflow
        let changed = false;

        // Horizontal
        if (horizontal === 'right' && finalX + popupWidth > graphWidth) {
          horizontal = 'left';
          changed = true;
        } else if (horizontal === 'left' && finalX < 0) {
          horizontal = 'right';
          changed = true;
        }

        // Vertical
        if (vertical === 'bottom' && finalY + popupHeight > graphHeight) {
          vertical = 'top';
          changed = true;
        } else if (vertical === 'top' && finalY < 0) {
          vertical = 'bottom';
          changed = true;
        }

        if (changed) setPopupAlignment({ vertical, horizontal });

        // Recalculate position based on alignment
        finalX = (horizontal === 'right')
          ? initialPos.x + 15
          : initialPos.x - popupWidth - 15;
        finalY = (vertical === 'bottom')
          ? initialPos.y + 15
          : initialPos.y - popupHeight - 15;

        // Clamp as fallback
        if (finalX < 5) finalX = 5;
        if (finalY < 5) finalY = 5;
        if (finalX + popupWidth > graphWidth - 5) finalX = graphWidth - popupWidth - 5;
        if (finalY + popupHeight > graphHeight - 5) finalY = graphHeight - popupHeight - 5;

        setFinalPopupPosition({ x: finalX, y: finalY });
        setIsPopupReady(true);
      };

      // Use a ResizeObserver to automatically reposition the popup when its size changes (e.g., switching tabs).
      const observer = new ResizeObserver(reposition);
      observer.observe(popupRef);

      onCleanup(() => {
        observer.disconnect();
      });
    } else {
      setIsPopupReady(false);
    }
  });

  useGraphMaximizeHotkeys(setMaximized);

  const { handleMaximize, handleRecenter, handleDownload } = createGraphToolbarHandlers(
      () => network,
      () => container,
      () => setMaximized((v: boolean) => !v)
  );

  return (
    <>
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
        onDownloadCSV={() => exportCommGraphToCSV(commGraphData, vm_states)}
        onDownload={handleDownload}
        isFullscreen={maximized()}
      />
        {popupVisible() && popupContent() && (
        <div
          ref={popupRef!}
          class="vm-state-popup"
          style={{
            position: "absolute",
            top: `${finalPopupPosition().y}px`,
            left: `${finalPopupPosition().x}px`,
            visibility: isPopupReady() ? 'visible' : 'hidden',
          }}
        >
          <VmStatePopup event={popupContent()!} />
        </div>
      )}
    </div>
    </>
  );
}
