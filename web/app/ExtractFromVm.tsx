


export const extractProgs = (vm_states) => {
    //returns list of program names contained in the first vm state of the execution
    let prog_list = vm_states[0].locals[0][0];
    if (!Array.isArray(prog_list)) {
        console.error("extractProgs : not an array :", prog_list);
        return [];
    }
    return prog_list.filter(obj => obj.hasOwnProperty('program')).map(obj => obj.program);

}