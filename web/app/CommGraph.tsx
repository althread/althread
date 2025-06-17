/** @jsxImportSource solid-js */
import vis from "vis-network/dist/vis-network.esm";
import { createSignal, onCleanup, onMount } from "solid-js"
import {nodeToString} from "./Node";
import GraphToolbar from "./GraphToolbar.jsx";

import { createGraphToolbarHandlers } from "./visHelpers";
import { useGraphMaximizeHotkeys } from "./hooks/useGraphMaximizeHotkeys";



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

const searchSenderNode = (graphNodes, receivingEvent, msgNum, broadcast) => {
  //searches for the sending event corresponding to the receiving event
  let suite = "";
  (broadcast) ? suite = "B" : suite = receivingEvent.receiver;
  let str = "p" + receivingEvent.sender + "_send" + "_to" + suite + "_" + msgNum;
  let sender_node = graphNodes.get(str);
  return sender_node;
  
}



export const renderMessageFlowGraph = (commGraphData, prog_list, vm_states) => {
  //returns div element to display the message flow graph and the vm states popup on click
  //prog_list = array of program names (strings)
  //commGraphData = array of communication events
  let container!: HTMLDivElement;
  let network: vis.Network | null = null;
  const [maximized, setMaximized] = createSignal(false);

  if (!commGraphData || commGraphData.length === 0) {
    return <pre>The communication graph will appear here (if any communication events are recorded).</pre>;
  }

  let [popupVisible, setPopupVisible] = createSignal(false);
  let [popupContent, setPopupContent] = createSignal("");
  let [popupPosition, setPopupPosition] = createSignal({ x: 0, y: 0 });

  onMount(() => {

    const nodes= new vis.DataSet();
    const edges = new vis.DataSet();
    const processLines = new Map(); //coordinates of start&end of each process line
    const processes: string[] = [];

    // extract processes name to make one line per process
      processes.push("main");
      prog_list.forEach(prog =>{
        processes.push(prog);
      });
    

    let ySpacing = 150; // space between processes vertically
    let xStart = -50; 
    let xEnd = 450; 
    //console.log(nodes);

    let nmsg_sent: number[] = [];
    let nb_process = 0; 

    // lines generation
    [...processes].forEach((processName, index) => {
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
      nb_process++;
      nmsg_sent.push(0);

      processLines.set(index, { start: startNode.id, end: endNode.id });
    });

    // message arrows 
    let i:number = 0;
    let broadcast: boolean = false;

    console.log("commGraphData", commGraphData);

    commGraphData.forEach(event => {
      let yposLine = 0;
      let id_txt = "p";
      let evt_type = String.fromCharCode(event.evt_type);
      
      //lengthen the process lines for each new event so it doesnt go overboard
      for(let j = 0; j<processes.length; ++j){
        nodes.update({id: `p${j}_end`, x: 450+20+50*i});
      }
      
      
      if ( evt_type === 'r') {
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
                      shape: "dot", size: 5, color: "#cccccc", vm_state: nodeToString(event.vm_state) };
      nodes.add(msgNode);
     
      if ((evt_type) === 'r'){
        let sender = searchSenderNode(nodes, event, event.number, broadcast);
        if (sender){
          edges.add({
            from: sender.id,
            to: msgNode.id,
            label: event.message,
            font:{
              size: 20,
              color: "white",
              align: "middle",
              background: "none",
              strokeWidth: 0,
            },
            arrows: "to",
            color: "green",
          })
        }
      }
      i++;
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
          let pos = event.pointer.DOM;
          setPopupContent(nodes.get(node_id).vm_state);
          setPopupPosition({x: pos.x, y: pos.y});
          setPopupVisible(true);
          previous_node_id = node_id;
          previous_node_colour = nodes.get(node_id).color;
          nodes.update({id: node_id, color: "#0080ff"});
          
        }
      } else {
          setPopupVisible(false);
      }
    }
    );

    onCleanup(() => { if (network) network.destroy(); });
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
        onDownload={handleDownload}
        isFullscreen={maximized()}
      />
        {popupVisible() && (
        <div
          style={{
            position: "absolute",
            top: `${popupPosition().y}px`,
            left: `${popupPosition().x}px`,
            background: "white",
            color: "black",
            padding: "0.1rem",
            border: "1px solid black",
            "box-shadow": "0px 2px 5px rgba(0, 0, 0, 0.3)",
            "z-index": "10000",
          }}
        >
          <pre style={{color: "black"}}>{popupContent()}</pre>
        </div>
      )}
    </div>
    </>
  );
}
