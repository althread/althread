// This file contains all the example programs embedded as strings
// Auto-generated from the examples/ directory
// Run 'npm run generate-examples' to regenerate

export interface ExampleInfo {
  fileName: string;
  title: string;
  description: string;
  tags: string[];
  content: string;
}

export const EXAMPLES: ExampleInfo[] = [
  {
    fileName: "concurrency.alt",
    title: "Concurrency",
    description: "Example program: Concurrency",
    tags: ["example","shared","concurrency","atomic","channels","communication","programs","functions","loops","conditionals","synchronization","messaging","math"],
    content: "shared {\n    let A = 1;\n    let B = 0;\n    let Start = false;\n    let WorkersFinished = 0;  // Counts finished workers\n}\n\nfn process_message(value: int, flag: bool) -> void {\n    print(\"Processing message: value=\" + value + \", flag=\" + flag);\n    atomic {\n        if flag {\n            A = value;\n        } else {\n            B = value;\n        }\n        WorkersFinished += 1; \n    }\n}\n\nfn verify_state() -> bool {\n    return (A == 125 && B == 125);\n}\n\nprogram Worker() {\n    await Start;\n    await receive in (x, y) => {\n        process_message(x, y);\n    };\n}\n\nmain {\n    let worker1 = run Worker();\n    let worker2 = run Worker();\n\n    channel self.out (int, bool)> worker1.in;\n    channel self.out2 (int, bool)> worker2.in;\n    \n    atomic { Start = true; }\n    \n    send out(125, true);\n    send out2(125, false);\n\n    // Waits for both workers to finish processing\n    await WorkersFinished == 2;\n\n    if verify_state() {\n        print(\"Channel test successful!\");\n    } else {\n        print(\"Channel test failed!\");\n    }\n}\n\n// Output:\n// Processing message: value=125, flag=true\n// Processing message: value=125, flag=false\n// Channel test successful!\n// or\n// Processing message: value=125, flag=false\n// Processing message: value=125, flag=true\n// Channel test successful!"
  },
  {
    fileName: "fibo.alt",
    title: "Fibo",
    description: "Example program: Fibo",
    tags: ["example","functions","loops","conditionals","recursion","fibonacci","math","algorithms"],
    content: "fn fibonacci_recursive(n: int, a: int, b: int) -> int {\n  if n == 0 {\n    return a;\n  } else {\n    return fibonacci_recursive(n - 1, b, a + b);\n  }\n}\n\nfn fibonacci_iterative(n: int, a: int, b: int) -> int {\n  for i in 1..n {\n    let c = a + b;\n    a = b;\n    b = c;\n  }\n  return b;\n}\n\nmain {\n    let n = 10;\n    let res = fibonacci_recursive(n, 0, 1);\n    print(\"Fibonacci recursive of \" + n + \": \" + res);\n\n    let res = fibonacci_iterative(n, 0, 1);\n    print(\"Fibonacci iterative of \" + n + \": \" + res);\n}\n\n// Outputs:\n// Fibonacci recursive of 10: 55\n// Fibonacci iterative of 10: 55"
  },
  {
    fileName: "list-of-list.alt",
    title: "List-Of-List",
    description: "Example program: List-Of-List",
    tags: ["example","loops"],
    content: "main {\n  let s: list(list(int));\n  for i in 0..10 {\n    let a : list(int);\n    for j in 0..i+1 {\n      a.push(j);\n    }\n    s.push(a);\n  }\n\n  for i in s {\n    print(i);\n    for j in i {\n      print(j);\n    }\n  }\n\n  let v = s.at(3);\n  let v = v.at(1);\n  print(\"=========\");\n  print(v);\n}"
  },
  {
    fileName: "loop-sum.alt",
    title: "Loop-Sum",
    description: "Example program: Loop-Sum",
    tags: ["example","shared","concurrency","programs","loops","math"],
    content: "shared {\n  let Sum = 0;\n}\n\nprogram A(my_id: int) {\n  loop {\n    Sum += 1;\n    Sum -= 1;\n  }\n}\n\neventually {\n  Sum == 2;\n}\n\nmain {\n  let n = 2;\n  let a:list(proc(A));\n  for i in 0..n {\n    let p = run A(i);\n  }\n}"
  },
  {
    fileName: "max.alt",
    title: "Max",
    description: "Example program: Max",
    tags: ["example","functions","conditionals","math"],
    content: "fn max(a: int, b: int) -> int {\n    if a > b {\n        return a;\n    }\n    return b;\n}\n\nmain {\n    print(\"The max between 5 and 10 is\", max(5, 10));\n}"
  },
  {
    fileName: "peterson_mutual_exlusion.alt",
    title: "Peterson Mutual Exlusion",
    description: "Example program: Peterson Mutual Exlusion",
    tags: ["example","shared","concurrency","programs","synchronization","math"],
    content: "shared {\n    const A_TURN = 1;\n    const B_TURN = 2;\n    let X: bool = false;\n    let Y: bool = false;\n    let T: int = 0;\n    let NbSC = 0;\n}\n\nprogram A() {\n    X = true;\n    T = B_TURN;\n\n    await Y == false || T == A_TURN;\n\n    NbSC += 1;\n    //section critique\n    NbSC -= 1;\n\n    X = false;\n    print(\"A is done\");\n}\n\nprogram B() {\n    Y = true;\n    T = A_TURN;\n    await X == false || T == B_TURN;\n\n    NbSC += 1;\n    //section critique\n    NbSC -= 1;\n\n    Y = false;\n    print(\"B is done\");\n}\n\nalways {\n    NbSC == 0 || NbSC == 1;\n}\n\nmain {\n    run A();\n    run B();\n}"
  },
  {
    fileName: "ring-election-eventually.alt",
    title: "Ring-Election-Eventually",
    description: "Example program: Ring-Election-Eventually",
    tags: ["example","shared","concurrency","channels","communication","programs","loops","conditionals","synchronization","messaging","math"],
    content: "shared {\n  let Done = false;\n  let Leader = 0;\n}\n\nprogram A(my_id: int) {\n\n  let leader_id = my_id;\n\n  send out(my_id);\n\n  loop atomic await receive in (x) => {\n    print(\"receive\", x);\n      if x > leader_id {\n        leader_id = x;\n        send out(x);\n      } else {\n        if x == leader_id {\n          print(\"finished\");\n          send out(x);\n          break;\n        }\n      }\n  };\n  \n  if my_id == leader_id {\n    print(\"I AM THE LEADER!!!\");\n    ! {\n        Done = true;\n        Leader += 1;\n    }\n  }\n}\n\neventually {\n    Leader == 1;\n}\n\n\nmain {\n  let n = 5;\n  let a:list(proc(A));\n  for i in 0..n {\n    let p = run A(i);\n    a.push(p);\n  }\n  for i in 0..n-1 {\n    let p1 = a.at(i);\n    let p2 = a.at(i+1);\n    channel p1.out (int)> p2.in;\n  }\n  \n  let p1 = a.at(n-1);\n  let p2 = a.at(0);\n  channel p1.out (int)> p2.in;\n\n  print(\"DONE\");\n}"
  },
  {
    fileName: "ring-election.alt",
    title: "Ring-Election",
    description: "Example program: Ring-Election",
    tags: ["example","shared","concurrency","channels","communication","programs","conditionals","synchronization","messaging","math"],
    content: "shared {\n  let Done = false;\n  let Leader = 0;\n}\n\nprogram A(my_id: int) {\n\n  let leader_id = my_id;\n\n  send out(my_id);\n\n  loop atomic await receive in (x) => {\n    print(\"receive\", x);\n      if x > leader_id {\n        leader_id = x;\n        send out(x);\n      } else {\n        if x == leader_id {\n          print(\"finished\");\n          send out(x);\n          break;\n        }\n      }\n  };\n  \n  if my_id == leader_id {\n    print(\"I AM THE LEADER!!!\");\n    ! {\n        Done = true;\n        Leader += 1;\n    }\n  }\n}\n\nalways {\n  !Done || (Leader == 1);\n}\n\nmain {\n  let a = run A(1);\n  let b = run A(2);\n\n  channel a.out (int)> b.in;\n  channel b.out (int)> a.in;\n\n  print(\"DONE\");\n}"
  },
  {
    fileName: "shared-list.alt",
    title: "Shared-List",
    description: "Example program: Shared-List",
    tags: ["example","shared","concurrency","atomic"],
    content: "shared {\n  let L:list(int);\n\n}\n\nmain {\n    \n\n    // add an element to a global list\n    // L.push() is not yet supported \n    //\n    atomic {\n      let l = L;\n      l.push(1);\n      l.push(2);\n      l.push(42);\n      L = l;\n    }\n    print(\"L = \", L);\n\n    // get an element from a global list\n    let a:int;\n    ! {\n        let l = L;\n        a = l.at(2);\n    }\n    print(\"a = \", a);\n}"
  },
  {
    fileName: "test-atomic.alt",
    title: "Test-Atomic",
    description: "Example program: Test-Atomic",
    tags: ["example","shared","concurrency","channels","communication","programs","synchronization","messaging","math"],
    content: "shared {\n  let A: bool = false;\n  let B: bool = true;\n  let Done = 0;\n}\n\nprogram A() {\n  print(\"starting A\");\n  ! {\n    A = false;\n    B = true;\n  }\n  Done += 1;\n  send out(42,true);\n}\n\nprogram B() {\n  print(\"starting B\");\n  ! {\n    A = true;\n    B = false;\n  }\n  Done += 1;\n}\n\nalways {\n  A || B;\n}\n\nmain {\n  let a = run A();\n  run B();\n  await Done == 2;\n\n  channel a.out (int, bool)> self.in;\n\n  await receive in(x,y) => {\n    print(\"Receive\", x, y);\n  };\n  print(\"DONE\");\n}"
  },
  {
    fileName: "test-break.alt",
    title: "Test-Break",
    description: "Example program: Test-Break",
    tags: ["example","shared","concurrency","programs","synchronization"],
    content: "shared {\n    let A = false;\n}\n\nprogram A() {\n    let i = 0;\n    loop {\n        await first {\n            A => {\n                print(\"A is true\");\n                break;\n            }\n            !A => {\n                print(\"A is false\");\n                break;\n            }\n        }\n    }\n}\n\nalways {\n}\n\nmain {\n    let a = run A();\n    print(\"started\", a);\n    A = true;\n}"
  },
  {
    fileName: "test-channels.alt",
    title: "Test-Channels",
    description: "Example program: Test-Channels",
    tags: ["example","shared","concurrency","channels","communication","programs","synchronization","messaging"],
    content: "shared {\n    let A = 1;\n    let B = 0;\n    let Start = false;\n}\nprogram A() {\n    await Start;\n    await receive in (x,y) => {\n        print(\"received \");\n    };\n}\n\nmain {\n    let pa = run A();\n    let pb = run A();\n\n    channel self.out (int, bool)> pa.in;\n    channel self.out2 (int, bool)> pb.in;\n    Start = true;\n    send out (125, true);\n    send out2 (125, false);\n\n}"
  },
  {
    fileName: "test-if-else.alt",
    title: "Test-If-Else",
    description: "Example program: Test-If-Else",
    tags: ["example","loops","conditionals"],
    content: "main {\n    for x in 0..5 {\n        if x == 1 {\n            print(\"x is 1\");\n        } else if x == 2 {\n            print(\"x is 2\");\n        } else if x == 3 {\n            print(\"x is 3\");\n        } else {\n            print(\"else\");\n        }\n    }\n}"
  },
  {
    fileName: "test-list.alt",
    title: "Test-List",
    description: "Example program: Test-List",
    tags: ["example","channels","communication","programs","loops","synchronization","messaging","math"],
    content: "main {\n    let n = 6;\n    let p: list(proc(A));\n    for i in 0..n {\n        let pid = run A(i);\n        p.push(pid);\n    }\n    \n    //not yet supported\n    //let p = [run A(i) for i in 0..10];\n\n    for i in 0..(n-1) {\n        let n = p.len();\n        let at_i = p.at(i);\n        let at_i2 = p.at((i+1)%n);\n        print(at_i, \"->\", at_i2);\n        channel at_i.out (int)> at_i2.in;\n    }\n\n    let first = p.at(0);\n    channel self.out (int)> first.in;\n    let n = p.len();\n    let last = p.at(n-1);\n    channel last.out (int)> self.in;\n\n    send out(0);\n\n    await receive in(i) => {\n        print(\"FINAL Received: \", i);\n    };\n}\n\n\nprogram A(id:int) {\n    print(\"Hello from A\");\n    await receive in (i) => {\n        id += i;\n        print(\"Received\", i, \" new value is \", id);\n    };\n    send out(id);\n}"
  },
  {
    fileName: "test-wait.alt",
    title: "Test-Wait",
    description: "Example program: Test-Wait",
    tags: ["example","shared","concurrency","programs","loops","conditionals","synchronization","ring"],
    content: "shared {\n    let VA = 1;\n}\n\nmain {\n\n    print(\"await first\");\n    //print \"CASE 1\" (because the keyword first is used)\n    await first {\n        (VA == 0) => { print(\"CASE 0\"); }\n        (VA == 1) => { print(\"CASE 1\"); VA = 2; }\n        (VA == 2) => { print(\"CASE 2\"); }\n    }\n    \n    print(\"await seq\");\n    VA = 1;\n    //print \"CASE 1\" and \"CASE 2\"\n    await seq {\n        (VA == 0) => { print(\"CASE 0\"); }\n        (VA == 1) => { print(\"CASE 1\"); VA = 2; }\n        (VA == 2) => { print(\"CASE 2\"); }\n    }\n    \n    \n    if VA == 0 {\n        print(\"if condition\");\n    }\n\n    VA = 0; // comment to see a deadlock\n    \n    await (VA == 0);\n    print(\"await condition\");\n}\n/**\n`condition` is a boolean expression\n`await condition` is a statement that waits for the condition to be true\n\n```\nfirst { \n    condition1 => block1,\n    condition2 => block2,\n}\n``` \nis an boolean expression that is true if one of the conditions is true. Each condition is evaluated sequentially from top to bottom, if one condition is true, it executes only the first corresponding block and then goes to the first instruction outside the block, hence:\n```\nawait first {\n    condition1 => block1,\n    condition2 => block2,\n}\n```\nwaits for one of the conditions to be true, then executes only the corresponding block, then continues with the rest of the program\n\nSimilarly, \n```\nseq { \n    condition1 => block1,\n    condition2 => block2,\n}\n```\nis an boolean expression that evaluates to true if one of the conditions is true, however here, when the block corresponding to the first true condition is executed, the remaining conditions are also evaluated, and the blocks associated with all the true conditions are executed sequentially from top to bottom. Hence,\n```\nawait seq {\n    condition1 => block1,\n    condition2 => block2,\n}\n```\nwaits for one of the conditions to be true, then executes the corresponding block, then evaluate the remaining conditions from this first true condition and execute all the blocks associated with true conditions. Then, the rest of the program continues.\n\nSince seq {} and first {} are boolean expressions, they can be used in if statements, while loops, and first/seq conditions.\n\nExample:\n```\nif seq {\n        first {\n            condition1 => block1,\n            condition2 => block2,\n        }\n        condition3 => block3,\n    } \n{\n    print(\"if seq\")\n}\n```\nmeans that if condition1 is true, block1 is executed, then condition3 is evaluated (if it is true, block3 is executed). Otherwise if condition1 is false, then condition2 is evaluated, if it is true, block2 is executed, then condition3 is evaluated (if it is true, block3 is executed), if condition2 is false, condition3 is evaluated, if it is true, block3 is executed. If all conditions are false, the entier expression is false and the pring statement is not executed.\n */"
  }
];

// Helper function to search examples by content and metadata
export function searchExamples(query: string): ExampleInfo[] {
  const searchTerm = query.toLowerCase().trim();
  
  if (!searchTerm) {
    return EXAMPLES;
  }
  
  return EXAMPLES.filter(example => {
    // Search in title, description, tags, filename, and content
    return (
      example.title.toLowerCase().includes(searchTerm) ||
      example.description.toLowerCase().includes(searchTerm) ||
      example.fileName.toLowerCase().includes(searchTerm) ||
      example.tags.some(tag => tag.toLowerCase().includes(searchTerm)) ||
      example.content.toLowerCase().includes(searchTerm)
    );
  });
}
