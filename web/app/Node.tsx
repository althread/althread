export type Node = {
  channels: Map<any, any[]>,
  globals: Map<any, any>,
  locals: { [key: string]: [any[], any] }
};

//////////////////////////////////////
export const node_entirely = (n: Node) => {
  return JSON.stringify(n, null, 2);
};
//////////////////////////////////////

export const literal = (value) => {
  if(Object.keys(value)[0] == "tuple") {
    return '('+(Object.values(value)[0] as any[]).map(literal).join(',')+')';
  }
  return value[Object.keys(value)[0]];//+'('+Object.values(value)[0]+')';
}

export const nodeToString = (n: Node) => {
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