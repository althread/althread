export const exportCommGraphToCSV = async (commGraphData: any[], vm_states?: any[]) => {
  // Helper function to escape CSV values properly
  const escapeCSV = (value: any): string => {
    if (value === null || value === undefined) return '';
    const str = String(value);
    // Always wrap in quotes and escape internal quotes
    return `"${str.replace(/"/g, '""').replace(/\n/g, ' ').replace(/\r/g, '')}"`;
  };

  const timestamp = new Date().toISOString().slice(0, 19).replace(/:/g, '-');

  // Export communication events
  const eventHeaders = [
    'Event_ID',
    'Event_Type',
    'Sender_PID',
    'Receiver_PID',
    'Message',
    'Sequence_Number',
    'Actor_Program',
    'Is_Broadcast'
  ];

  const eventRows = commGraphData.map((event, index) => {
    const eventType = String.fromCharCode(event.evt_type);
    const isBroadcast = event.receiver === undefined || event.receiver === null;
    
    return [
      escapeCSV(index),
      escapeCSV(eventType === 's' ? 'Send' : 'Receive'),
      escapeCSV(event.sender),
      escapeCSV(isBroadcast ? 'Broadcast' : event.receiver),
      escapeCSV(event.message),
      escapeCSV(event.number),
      escapeCSV(event.actor_prog_name),
      escapeCSV(isBroadcast)
    ];
  });
  
  const eventsCSV = [eventHeaders, ...eventRows]
    .map(row => row.join(','))
    .join('\n');

  // Export process summary
  const processHeaders = ['Process_ID', 'Program_Name', 'Send_Count', 'Receive_Count', 'Total_Events'];
  const processMap = new Map();

  commGraphData.forEach(event => {
    const eventType = String.fromCharCode(event.evt_type);
    const pid = eventType === 's' ? event.sender : event.receiver;
    
    if (!processMap.has(pid)) {
      processMap.set(pid, {
        program: event.actor_prog_name,
        sends: 0,
        receives: 0
      });
    }
    
    if (eventType === 's') {
      processMap.get(pid).sends++;
    } else {
      processMap.get(pid).receives++;
    }
  });

  const processRows = Array.from(processMap.entries()).map(([pid, stats]) => [
    escapeCSV(pid),
    escapeCSV(stats.program),
    escapeCSV(stats.sends),
    escapeCSV(stats.receives),
    escapeCSV(stats.sends + stats.receives)
  ]);

  const processCSV = [processHeaders, ...processRows]
    .map(row => row.join(','))
    .join('\n');

  // Create zip with files
  try {
    const JSZipModule = await import('jszip');
    const JSZip = JSZipModule.default || JSZipModule;
    const zip = new JSZip();

    // Add files to zip
    zip.file(`comm-events-${timestamp}.csv`, eventsCSV);
    zip.file(`process-summary-${timestamp}.csv`, processCSV);
    
    // Add a README file with metadata
    const sendEvents = commGraphData.filter(event => String.fromCharCode(event.evt_type) === 's');
    const receiveEvents = commGraphData.filter(event => String.fromCharCode(event.evt_type) === 'r');
    
    const readme = `Althread Communication Graph Export
Generated: ${new Date().toISOString()}
Total Events: ${commGraphData.length}
Total Processes: ${processMap.size}
Send Events: ${sendEvents.length}
Receive Events: ${receiveEvents.length}

Files:
- comm-events-${timestamp}.csv: All communication events chronologically
- process-summary-${timestamp}.csv: Summary statistics per process

Format: CSV with UTF-8 encoding
Event types: 'Send' for outgoing messages, 'Receive' for incoming messages`;
    
    zip.file('README.txt', readme);
    
    // Generate and download zip
    const content = await zip.generateAsync({ type: 'blob' });
    const url = URL.createObjectURL(content);
    const a = document.createElement('a');
    a.href = url;
    a.download = `althread-commgraph-${timestamp}.zip`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    
  } catch (error) {
    console.error('Failed to create zip file:', error);
    console.log('Falling back to separate file downloads...');
    
    // Fallback to separate files
    [
      { name: `comm-events-${timestamp}.csv`, content: eventsCSV },
      { name: `process-summary-${timestamp}.csv`, content: processCSV }
    ].forEach((file, index) => {
      setTimeout(() => {
        const blob = new Blob([file.content], { type: 'text/csv;charset=utf-8' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = file.name;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
      }, index * 200);
    });
  }
};

// Keep the existing state graph export function
export const exportStatesToCSV = async (nodes: any[], edges: any[]) => {
  // Helper function to escape CSV values properly
  const escapeCSV = (value: any): string => {
    if (value === null || value === undefined) return '';
    const str = String(value);
    // Always wrap in quotes and escape internal quotes
    return `"${str.replace(/"/g, '""').replace(/\n/g, ' ').replace(/\r/g, '')}"`;
  };

  const timestamp = new Date().toISOString().slice(0, 19).replace(/:/g, '-');

  // Export nodes
  const nodeHeaders = [
    'Node_ID',
    'Level', 
    'Is_Violation_Node',
    'Program_Count',
    'Global_Variables',
    'Active_Channels_Summary'
  ];

  const nodeRows = nodes.map(node => {
    // Extract structured information from the node label
    const label = node.label || '';
    
    // Count programs - look for program headers
    const programMatches = label.match(/\*Program [^*]+\*/g) || [];
    const programCount = programMatches.length;
    
    // Extract globals more carefully
    const globalsSection = label.match(/\*Globals:\*\n(.*?)\n\n\*Program States/s);
    let globalVars = 'None';
    if (globalsSection) {
      const globalLines = globalsSection[1]
        .split('\n')
        .map(line => line.trim())
        .filter(line => line && !line.includes('_No global variables_'))
        .map(line => line.replace(/^\s*/, ''));
      globalVars = globalLines.length > 0 ? globalLines.join('; ') : 'None';
    }
    
    // Extract channel information more carefully - just count active channels
    const channelMatches = label.match(/(\w+) <- ([^"]*?)(?=\n|$)/g) || [];
    const activeChannelsCount = channelMatches.length;
    const channelsSummary = activeChannelsCount > 0 ? 
      `${activeChannelsCount} active channels` : 'No active channels';

    return [
      escapeCSV(node.id),
      escapeCSV(node.level || ''),
      escapeCSV(node.isViolationNode || false),
      escapeCSV(programCount),
      escapeCSV(globalVars),
      escapeCSV(channelsSummary)
    ];
  });
  
  const nodeCSV = [nodeHeaders, ...nodeRows]
    .map(row => row.join(','))
    .join('\n');

  // Export edges
  const edgeHeaders = ['From_Node', 'To_Node', 'Transition_Label', 'Program', 'Process_ID', 'Instructions'];
  const edgeRows = edges.map(edge => {
    const label = edge.label || '';
    
    // Parse transition label (e.g., "A#1: 11" or "main#0: 26,17")
    const transitionMatch = label.match(/([^#]+)#(\d+):\s*(.+)/);
    const program = transitionMatch ? transitionMatch[1] : '';
    const processId = transitionMatch ? transitionMatch[2] : '';
    const instructions = transitionMatch ? transitionMatch[3] : label;
    
    return [
      escapeCSV(edge.from),
      escapeCSV(edge.to),
      escapeCSV(label),
      escapeCSV(program),
      escapeCSV(processId),
      escapeCSV(instructions)
    ];
  });
  
  const edgeCSV = [edgeHeaders, ...edgeRows]
    .map(row => row.join(','))
    .join('\n');

  // create zip with files
  try {
    const JSZipModule = await import('jszip');
    const JSZip = JSZipModule.default || JSZipModule;
    const zip = new JSZip();

  // Add files to zip
    zip.file(`nodes-${timestamp}.csv`, nodeCSV);
    zip.file(`edges-${timestamp}.csv`, edgeCSV);
    
    // Add a README file with metadata
    const readme = `Althread State Graph Export
Generated: ${new Date().toISOString()}
Total Nodes: ${nodes.length}
Total Edges: ${edges.length}

Files:
- nodes-${timestamp}.csv: Node data with state information
- edges-${timestamp}.csv: Edge data with transitions

Format: CSV with UTF-8 encoding`;
    
    zip.file('README.txt', readme);
    
    // Generate and download zip
    zip.generateAsync({ type: 'blob' }).then(content => {
      const url = URL.createObjectURL(content);
      const a = document.createElement('a');
      a.href = url;
      a.download = `althread-graph-${timestamp}.zip`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    });
  } catch(error) {
    console.error('Failed to create zip file:', error);
    // Fallback to separate files if JSZip is not available
    console.log('Falling back to separate file downloads...');
    // ... original separate file download code here as fallback
  };
};
