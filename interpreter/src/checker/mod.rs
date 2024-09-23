

pub fn check_program(compiled_project: &'a CompiledProject) -> Result<(), String> {


    let mut state_graph: HashMap<althread::vm::VM, Vec<&althread::vm::VM>> = HashSet::new();

    let mut next_nodes = Vec::new();

    next_nodes.push(althread::vm::VM::new(&compiled_project));

    while !next_nodes.is_empty() {
        let current_node = next_nodes.pop().unwrap();
        let mut next_nodes = todo!();
        /*
        let info = vm.next().unwrap_or_else(|err| {
            err.report(&source);
            exit(1);
        });
        */
        for n in next_nodes {
            if !state_graph.contains_key(&n) {
                state_graph.insert(n, Vec::new());
                next_nodes.push(n);
            }
        }
        state_graph.insert(current_node, next_nodes);
    }
}