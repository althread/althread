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
    fileName: "coffee_machine.alt",
    title: "Coffee Machine",
    description: "Example program: Coffee Machine",
    tags: ["example","shared","concurrency","programs","loops","conditionals","synchronization","math"],
    content: "shared {\n    let Coin_inserted = false;\n    let Coffee_ready = false;\n    let State = 0; // 0: Idle, 1: Brewing, 2: Done\n}\n\nprogram Machine() {\n    loop {\n         // Wait for coin\n         await Coin_inserted;\n         \n         // Consume coin\n         Coin_inserted = false;\n         \n         // Start brewing\n         State = 1; \n         \n         // Finish brewing\n         State = 2; \n         Coffee_ready = true;\n         \n         // Wait for user to take coffee\n         await !Coffee_ready;\n         \n         // Return to idle\n         State = 0; \n    }\n}\n\nprogram User() {\n    loop {\n        // Action: Insert coin\n        // We can only insert if machine is ready (optional constraint, but good for flow)\n        // await state == 0; \n        Coin_inserted = true;\n        \n        // Action: Wait for coffee\n        await Coffee_ready;\n        \n        // Action: Take coffee\n        Coffee_ready = false;\n    }\n}\n\ncheck {\n    // Response property: If I put a coin, I eventually get coffee\n    // Note: This relies on fairness (Machine must be scheduled)\n    always ( if Coin_inserted { eventually Coffee_ready } );\n}\n\ncheck {\n    // Safety: Coffee is never ready in Idle state\n    always ( if (State == 0) { !Coffee_ready } );\n}\n\nmain {\n    run Machine();\n    run User();\n}"
  },
  {
    fileName: "concurrency.alt",
    title: "Concurrency",
    description: "Example program: Concurrency",
    tags: ["example","shared","concurrency","atomic","channels","communication","programs","functions","loops","conditionals","synchronization","messaging","math"],
    content: "shared {\n    let A = 1;\n    let B = 0;\n    let Start = false;\n    let WorkersFinished = 0;  // Counts finished workers\n}\n\nfn process_message(value: int, flag: bool) -> void {\n    print(\"Processing message: value=\" + value + \", flag=\" + flag);\n    atomic {\n        if flag {\n            A = value;\n        } else {\n            B = value;\n        }\n        WorkersFinished = WorkersFinished + 1; \n    }\n}\n\nfn verify_state() -> bool {\n    return (A == 125 && B == 125);\n}\n\nprogram Worker() {\n    await Start;\n    \n    await receive in (x, y);\n    \n    process_message(x, y);\n}\n\nmain {\n    let worker1 = run Worker();\n    let worker2 = run Worker();\n\n    channel self.out (int, bool)> worker1.in;\n    channel self.out2 (int, bool)> worker2.in;\n    \n    atomic { Start = true; }\n    \n    send out(125, true);\n    send out2(125, false);\n\n    // Waits for both workers to finish processing\n    await WorkersFinished == 2;\n\n    if verify_state() {\n        print(\"Channel test successful!\");\n    } else {\n        print(\"Channel test failed!\");\n    }\n}\n\n// Output:\n// Processing message: value=125, flag=true\n// Processing message: value=125, flag=false\n// Channel test successful!\n// or\n// Processing message: value=125, flag=false\n// Processing message: value=125, flag=true\n// Channel test successful!"
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
    tags: ["example","shared","concurrency","programs","loops","conditionals","math"],
    content: "shared {\n  let Sum = 0;\n}\n\nprogram A(my_id: int) {\n  loop {\n    Sum = Sum + 1;\n    Sum = Sum - 1;\n  }\n}\n\ncheck {\n  always if Sum == 2 { eventually Sum == 1 };\n}\n\nmain {\n  let n = 2;\n  let a:list(proc(A));\n  for i in 0..n {\n    let p = run A(i);\n  }\n}"
  },
  {
    fileName: "ltl-always-eventually-fail.alt",
    title: "Ltl-Always-Eventually-Fail",
    description: "Example program: Ltl-Always-Eventually-Fail",
    tags: ["example","shared","concurrency","atomic","programs"],
    content: "shared {\n    let Tick: int = 0;\n}\n\nprogram TickOnce() {\n    atomic {\n        Tick = Tick + 1;\n    }\n}\n\ncheck {\n    always eventually (Tick > 1);\n}\n\nmain {\n    run TickOnce();\n}"
  },
  {
    fileName: "ltl-always-eventually.alt",
    title: "Ltl-Always-Eventually",
    description: "Example program: Ltl-Always-Eventually",
    tags: ["example","shared","concurrency","atomic","programs"],
    content: "shared {\n    let Tick: int = 0;\n}\n\nprogram InfiniteTicker() {\n    atomic {\n        Tick = Tick + 1;\n    }\n}\n\ncheck {\n    always eventually (Tick > 0);\n}\n\nmain {\n    run InfiniteTicker();\n}"
  },
  {
    fileName: "ltl-boolean-workflow.alt",
    title: "Ltl-Boolean-Workflow",
    description: "Example program: Ltl-Boolean-Workflow",
    tags: ["example","shared","concurrency","programs","ring"],
    content: "// Example 5: Boolean conditions\n// Testing various boolean combinations\n\nshared {\n    let Ready: bool = false;\n    let Started: bool = false;\n    let Finished: bool = false;\n}\n\nprogram Workflow() {\n    Ready = true;\n    Started = true;\n    Finished = true;\n}\n\n// All these formulas PASS with the workflow\ncheck {\n    eventually (Ready == true);\n}\n\ncheck {\n    always (!(Ready == true) || eventually (Started == true));\n}\n\ncheck {\n    eventually (Finished == true);\n}\n\n// This demonstrates ordering: Ready before Started before Finished\n// In actual execution, they happen in sequence\n// check {\n//     always (!(Started == true) || (Ready == true))\n// }\n\nmain {\n    run Workflow();\n}"
  },
  {
    fileName: "ltl-bounded-buffer.alt",
    title: "Ltl-Bounded-Buffer",
    description: "Example program: Ltl-Bounded-Buffer",
    tags: ["example","shared","concurrency","programs","conditionals","math"],
    content: "// Example 7: Bounded buffer property\n// Buffer should never overflow or underflow\n\nshared {\n    let BufferSize: int = 0;\n    let MAX_SIZE: int = 10;\n}\n\nprogram Producer() {\n    if (BufferSize < MAX_SIZE) {\n        BufferSize = BufferSize + 1;\n    }\n}\n\nprogram Consumer() {\n    if (BufferSize > 0) {\n        BufferSize = BufferSize - 1;\n    }\n}\n\n// These formulas PASS: buffer stays within bounds\ncheck {\n    always (BufferSize >= 0);\n}\n\ncheck {\n    always (BufferSize <= MAX_SIZE);\n}\n\n// Combined: buffer is always in valid range\ncheck {\n    always (BufferSize >= 0 && BufferSize <= MAX_SIZE);\n}\n\nmain {\n    run Producer();\n    run Producer();\n    run Consumer();\n    run Producer();\n}"
  },
  {
    fileName: "ltl-broadcast-fail.alt",
    title: "Ltl-Broadcast-Fail",
    description: "Example program: Ltl-Broadcast-Fail",
    tags: ["example","channels","communication","programs","loops","synchronization","messaging"],
    content: "program sender() {\n    send out.a.*(42);\n}\n\nprogram receiver() {\n    await receive in(x) => {\n        print(\"receive\", x);\n    };\n}\n\nmain {\n    let s = run sender();\n    let r1 = run receiver();\n    let r2 = run receiver();\n    \n    channel s.out.a.a (int)> r1.in;\n    channel s.out.b.b (int)> r2.in;\n}\n\ncheck {\n    for p in $.procs.receiver { eventually p.reaches(end) };\n}"
  },
  {
    fileName: "ltl-broadcast-pass.alt",
    title: "Ltl-Broadcast-Pass",
    description: "Example program: Ltl-Broadcast-Pass",
    tags: ["example","channels","communication","programs","loops","synchronization","messaging"],
    content: "program sender() {\n    send out.*(42);\n}\n\nprogram receiver() {\n    label L;\n    await receive in(x);\n    print(\"receive\", x);\n}\n\nmain {\n    let s = run sender();\n    let r1 = run receiver();\n    let r2 = run receiver();\n    \n    channel s.out.a.a (int)> r1.in;\n    channel s.out.b.b (int)> r2.in;\n}\n\ncheck {\n    for p in $.procs.receiver { eventually p.reaches(end) };\n}"
  },
  {
    fileName: "ltl-deadlock-freedom.alt",
    title: "Ltl-Deadlock-Freedom",
    description: "Example program: Ltl-Deadlock-Freedom",
    tags: ["example","shared","concurrency","programs","conditionals","ring"],
    content: "// Example 10: Deadlock freedom\n// System should eventually make progress\n\nshared {\n    let Lock1: bool = false;\n    let Lock2: bool = false;\n    let Progress: int = 0;\n}\n\nprogram UseResources() {\n    // Acquire locks in consistent order (prevents deadlock)\n    Lock1 = true;\n    Lock2 = true;\n    \n    // Do work\n    Progress = Progress + 1;\n    \n    // Release locks\n    Lock2 = false;\n    Lock1 = false;\n}\n\n// These formulas PASS: system makes progress\ncheck {\n    eventually (Progress > 0);\n}\n\ncheck {\n    always (Progress >= 0);\n}\n\n// Locks are eventually released\ncheck {\n    always eventually (Lock1 == false && Lock2 == false);\n}\n\n// This would demonstrate deadlock if we had two processes\n// with inconsistent lock ordering\n// program DeadlockProne() {\n//     Lock2 = true;\n//     Lock1 = true;  // Opposite order!\n//     progress = progress + 1;\n//     lock1 = false;\n//     lock2 = false;\n// }\n\nmain {\n    run UseResources();\n}"
  },
  {
    fileName: "ltl-eventually-simple.alt",
    title: "Ltl-Eventually-Simple",
    description: "Example program: Ltl-Eventually-Simple",
    tags: ["example","shared","concurrency","programs"],
    content: "// Example 3: Liveness property with eventually\n// Property: Something good eventually happens\n\nshared {\n    let Flag: bool = false;\n    let Counter: int = 0;\n}\n\nprogram EventuallySetsFlag() {\n    Counter = Counter + 1;\n    Counter = Counter + 1;\n    Flag = true;\n}\n\n// This formula PASSES: Flag eventually becomes true\n\ncheck {\n    eventually (Flag == true);\n}\n\n// This formula also PASSES: Counter eventually exceeds 1\ncheck {\n    eventually (Counter > 1);\n}\n\nmain {\n    run EventuallySetsFlag();\n}"
  },
  {
    fileName: "ltl-implication-fail.alt",
    title: "Ltl-Implication-Fail",
    description: "Example program: Ltl-Implication-Fail",
    tags: ["example","shared","concurrency","programs","conditionals"],
    content: "shared {\n    let Request: bool = false;\n    let Granted: bool = false;\n    let Completed: bool = false;\n}\n\nprogram RequestGrantComplete() {\n        Request = true;\n        Granted = true;\n        Completed = true;\n}\ncheck {\n    always (if Request { Granted });\n}\n\nmain {\n    run RequestGrantComplete();\n}"
  },
  {
    fileName: "ltl-implications.alt",
    title: "Ltl-Implications",
    description: "Example program: Ltl-Implications",
    tags: ["example","shared","concurrency","atomic","programs","loops","conditionals","synchronization"],
    content: "// Example 11: Complex conditions with implications\n// If something happens, then something else must follow\n\nshared {\n    let Request: bool = false;\n    let Granted: bool = false;\n    let Completed: bool = false;\n}\n\nprogram RequestGrantComplete() {\n    loop {\n        await Request;\n\n        Granted = true;\n\n        Completed = true;\n\n        await Completed == false; // commenting this line make the check fails\n    }\n}\n\nprogram MakeRequest() {\n    loop {\n        // Make a request\n        Request = true;\n\n        await Completed;\n\n        print(\"Request completed.\");\n\n        // Reset for next iteration\n        atomic {\n            Granted = false;\n            Completed = false;\n            Request = false;\n        }\n    }\n}\n\n// These formulas demonstrate implications\ncheck {\n    always (if Request { eventually Granted });\n    \n    always (if Granted { eventually Completed });\n    \n    eventually Request;\n    \n    always (if Completed { Granted });\n}\n\nmain {\n    run RequestGrantComplete();\n    run MakeRequest();\n}"
  },
  {
    fileName: "ltl-multiple-properties.alt",
    title: "Ltl-Multiple-Properties",
    description: "Example program: Ltl-Multiple-Properties",
    tags: ["example","shared","concurrency","programs","math"],
    content: "// Example 12: Multiple properties on same system\n// Demonstrates checking several properties at once\n\nshared {\n    let Value: int = 0;\n    let PositiveCount: int = 0;\n    let NegativeCount: int = 0;\n}\n\nprogram Modifier() {\n    Value = Value + 5;\n    PositiveCount = PositiveCount + 1;\n    \n    Value = Value - 3;\n    \n    Value = Value + 2;\n    PositiveCount = PositiveCount + 1;\n}\n\n// Property 1: Value eventually positive\ncheck {\n    eventually (Value > 0);\n}\n\n// Property 2: Positive operations counted\ncheck {\n    eventually (PositiveCount > 0);\n}\n\n// Property 3: Final value is sum of operations\ncheck {\n    eventually (Value == 4);  // 5 - 3 + 2 = 4\n}\n\n// Property 4: Counters are non-negative\ncheck {\n    always (PositiveCount >= 0);\n}\n\ncheck {\n    always (NegativeCount >= 0);\n}\n\n// Property 5: Value changes monotonically upward overall\n// (This depends on specific execution)\n// check {\n//     always (Value >= 0)  // Would FAIL: value becomes -3 then 2\n// }\n\nmain {\n    run Modifier();\n}"
  },
  {
    fileName: "ltl-mutual-exclusion.alt",
    title: "Ltl-Mutual-Exclusion",
    description: "Example program: Ltl-Mutual-Exclusion",
    tags: ["example","shared","concurrency","programs"],
    content: "// Example 6: Mutual exclusion property\n// Two processes should never be in critical section simultaneously\n\nshared {\n    let Cs1: bool = false;  // Process 1 in critical section\n    let Cs2: bool = false;  // Process 2 in critical section\n    let Turn: int = 1;\n}\n\nprogram Process1() {\n    // Enter critical section\n    Cs1 = true;\n    \n    // Critical section work\n    Turn = 2;\n    \n    // Exit critical section\n    Cs1 = false;\n}\n\nprogram Process2() {\n    // Enter critical section\n    Cs2 = true;\n    \n    // Critical section work\n    Turn = 1;\n    \n    // Exit critical section\n    Cs2 = false;\n}\n\n// This formula FAILS be\ncheck {\n    always (!(Cs1 == true && Cs2 == true));\n}\n\n\nmain {\n    run Process1();\n    run Process2();\n}"
  },
  {
    fileName: "ltl-resource-allocation.alt",
    title: "Ltl-Resource-Allocation",
    description: "Example program: Ltl-Resource-Allocation",
    tags: ["example","shared","concurrency","programs","conditionals"],
    content: "// Example 9: Resource allocation\n// Resource count should never exceed available resources\n\nshared {\n    let Allocated: int = 0;\n    let MAX_RESOURCES: int = 5;\n}\n\nprogram Allocate() {\n    if (Allocated < MAX_RESOURCES) {\n        Allocated = Allocated + 1;\n    }\n}\n\nprogram Release() {\n    if (Allocated > 0) {\n        Allocated = Allocated - 1;\n    }\n}\n\n// These formulas PASS: resources managed correctly\ncheck {\n    always (Allocated >= 0);\n}\n\ncheck {\n    always (Allocated <= MAX_RESOURCES);\n}\n\n// Resource usage stays bounded\ncheck {\n    always (Allocated >= 0 && Allocated <= MAX_RESOURCES);\n}\n\n// This would FAIL without the guard in Allocate\n// If we had: Allocated = Allocated + 1; (without if)\n// check {\n//     always (Allocated <= MAX_RESOURCES)\n// }\n\nmain {\n    run Allocate();\n    run Allocate();\n    run Allocate();\n    run Release();\n    run Allocate();\n}"
  },
  {
    fileName: "ltl-safety-simple.alt",
    title: "Ltl-Safety-Simple",
    description: "Example program: Ltl-Safety-Simple",
    tags: ["example","shared","concurrency","programs"],
    content: "// Example 1: Simple safety property\n// Property: x should always be non-negative\n\nshared {\n    let X: int = 0;\n}\n\nprogram Incrementer() {\n    X = X + 1;\n}\n\nprogram Decrementer() {\n    X = X - 1;\n}\n\n// This formula PASSES: X starts at 0 and is only incremented\ncheck {\n    always eventually (X >= 0);\n}\n\nmain {\n    run Incrementer();\n    run Incrementer();\n    run Incrementer();\n    // If we uncomment this line, the check above would FAIL (X becomes negative)\n    // run Decrementer();\n}"
  },
  {
    fileName: "ltl-safety-violation.alt",
    title: "Ltl-Safety-Violation",
    description: "Example program: Ltl-Safety-Violation",
    tags: ["example","shared","concurrency","programs"],
    content: "// Example 2: Safety violation\n// Demonstrates a counter that can become negative\n\nshared {\n    let Counter: int = 5;\n}\n\nprogram BadDecrement() {\n    Counter = Counter - 10;\n}\n\n// This formula FAILS: Counter becomes -5 after BadDecrement\ncheck {\n    always (Counter >= 0);\n}\n\n// This formula PASSES: Counter stays above -10\n// check {\n//     always (Counter >= -10)\n// }\n\nmain {\n    run BadDecrement();\n}"
  },
  {
    fileName: "ltl-state-machine.alt",
    title: "Ltl-State-Machine",
    description: "Example program: Ltl-State-Machine",
    tags: ["example","shared","concurrency","programs","conditionals"],
    content: "// Example 8: State machine property\n// System progresses through well-defined states\n\nshared {\n    let State: int = 0;  // 0=INIT, 1=READY, 2=RUNNING, 3=DONE\n}\n\nprogram StateMachine() {\n    // INIT -> READY\n    State = 1;\n    \n    // READY -> RUNNING\n    State = 2;\n    \n    // RUNNING -> DONE\n    State = 3;\n}\n\n// These formulas PASS: State progresses correctly\ncheck {\n    eventually (State == 1);  // Eventually reaches READY\n}\n\ncheck {\n    eventually (State == 3);  // Eventually reaches DONE\n}\n\n// State never goes backwards (monotonic)\ncheck {\n    always (State >= 0 && State <= 3);\n}\n\n// This would FAIL if we had: State = 1; State = 0;\n// check {\n//     always (!(State == 1) || eventually (State == 2))\n// }\n\nmain {\n    run StateMachine();\n}"
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
    content: "shared {\n    const A_TURN = 1;\n    const B_TURN = 2;\n    let X: bool = false;\n    let Y: bool = false;\n    let T: int = 0;\n    let NbSC = 0;\n}\n\nprogram A() {\n    X = true;\n    T = B_TURN;\n\n    await Y == false || T == A_TURN;\n\n    NbSC = NbSC + 1;\n    //section critique\n    NbSC = NbSc - 1;\n\n    X = false;\n    print(\"A is done\");\n}\n\nprogram B() {\n    Y = true;\n    T = A_TURN;\n    await X == false || T == B_TURN;\n\n    NbSC = NbSc + 1;\n    //section critique\n    NbSC = NbSc - 1;\n\n    Y = false;\n    print(\"B is done\");\n}\n\nalways {\n    NbSC == 0 || NbSC == 1;\n}\n\nmain {\n    run A();\n    run B();\n}"
  },
  {
    fileName: "readers_writers.alt",
    title: "Readers Writers",
    description: "Example program: Readers Writers",
    tags: ["example","shared","concurrency","atomic","programs","conditionals","synchronization"],
    content: "shared {\n    let Readers = 0;\n    let Writers = 0;\n}\n\nprogram Reader() {\n    loop {\n        atomic { \n            // Wait until no writer is active\n            await Writers == 0;\n            // Enter reading section\n            Readers = Readers + 1; \n        }\n        \n        // Reading happens here...\n        \n        // Exit reading section\n        atomic { Readers = Readers - 1; }\n    }\n}\n\nprogram Writer() {\n    loop {\n        \n        // Enter writing section\n        atomic { \n            // Wait until no readers and no other writers\n            await (Readers == 0 && Writers == 0);\n            \n            Writers = Writers + 1; \n        }\n        \n        // Writing happens here...\n        \n        // Exit writing section\n        atomic { Writers = Writers - 1; }\n    }\n}\n\ncheck {\n    // Safety: Never more than one writer\n    always ( Writers <= 1 );\n}\n\ncheck {\n    // Safety: If a writer is active, there are no readers\n    always ( if (Writers > 0) { (Readers == 0) } );\n}\n\ncheck {\n    // Safety: If readers are active, there is no writer\n    always ( if (Readers > 0) { (Writers == 0) } );\n}\n\nmain {\n    // One Writer, Two Readers\n    run Reader();\n    run Reader();\n    run Writer();\n}"
  },
  {
    fileName: "ring-election-eventually.alt",
    title: "Ring-Election-Eventually",
    description: "Example program: Ring-Election-Eventually",
    tags: ["example","shared","concurrency","channels","communication","programs","loops","conditionals","synchronization","messaging","math"],
    content: "shared {\n  let Leader = 0;\n}\n\nprogram A(my_id: int) {\n\n  let leader_id = my_id;\n\n  send out(my_id);\n\n  loop atomic await receive in (x) => {\n    print(\"receive\", x);\n      if x > leader_id {\n        leader_id = x;\n        send out(x);\n      } else {\n        if x == leader_id {\n          print(\"finished\");\n          send out(x);\n          break;\n        }\n      }\n  }\n  \n  label L;\n  if my_id == leader_id {\n    print(\"I AM THE LEADER!!!\");\n    @ {\n        Leader = Leader + 1;\n    }\n  }\n}\n\ncheck {\n  for p in $.procs.A { eventually p.reaches(L) };\n}\nalways {\n  Leader <= 1;\n}\n\n\nmain {\n  let n = 3;\n  let a:list(proc(A));\n  for i in 0..n {\n    let p = run A(i);\n    a.push(p);\n  }\n  for i in 0..n-1 {\n    let p1 = a.at(i);\n    let p2 = a.at(i+1);\n    channel p1.out (int)> p2.in;\n  }\n  \n  let p1 = a.at(n-1);\n  let p2 = a.at(0);\n  channel p1.out (int)> p2.in;\n}"
  },
  {
    fileName: "ring-election.alt",
    title: "Ring-Election",
    description: "Example program: Ring-Election",
    tags: ["example","shared","concurrency","channels","communication","programs","conditionals","synchronization","messaging","math"],
    content: "shared {\n  let Done = false;\n  let Leader = 0;\n}\n\nprogram A(my_id: int) {\n\n  let leader_id = my_id;\n\n  send out(my_id);\n\n  loop atomic await receive in (x) => {\n    print(\"receive\", x);\n    if x > leader_id {\n      leader_id = x;\n      send out(x);\n    } else {\n      if x == leader_id {\n        print(\"finished\");\n        send out(x);\n        break;\n      }\n    }\n  }\n  \n  if my_id == leader_id {\n    print(\"I AM THE LEADER!!!\");\n    @ {\n        Done = true;\n        Leader = Leader + 1;\n    }\n  }\n}\n\nalways {\n  !Done || (Leader == 1);\n}\n\nmain {\n  let a = run A(1);\n  let b = run A(2);\n\n  channel a.out (int)> b.in;\n  channel b.out (int)> a.in;\n\n  print(\"DONE\");\n}"
  },
  {
    fileName: "shared-list.alt",
    title: "Shared-List",
    description: "Example program: Shared-List",
    tags: ["example","shared","concurrency","atomic"],
    content: "shared {\n  let L:list(int);\n\n}\n\nmain {\n    \n\n    // add an element to a global list\n    // L.push() is not yet supported \n    //\n    atomic {\n      let l = L;\n      l.push(1);\n      l.push(2);\n      l.push(42);\n      L = l;\n    }\n    print(\"L = \", L);\n\n    // get an element from a global list\n    let a:int;\n    @ {\n        let l = L;\n        a = l.at(2);\n    }\n    print(\"a = \", a);\n}"
  },
  {
    fileName: "test-atomic.alt",
    title: "Test-Atomic",
    description: "Example program: Test-Atomic",
    tags: ["example","shared","concurrency","channels","communication","programs","synchronization","messaging","math"],
    content: "shared {\n  let A: bool = false;\n  let B: bool = true;\n  let Done = 0;\n}\n\nprogram A() {\n  print(\"starting A\");\n  @ {\n    A = false;\n    B = true;\n  }\n  Done = Done + 1;\n  send out(42,true);\n}\n\nprogram B() {\n  print(\"starting B\");\n  @ {\n    A = true;\n    B = false;\n  }\n  Done = Done + 1;\n}\n\nalways {\n  A || B;\n}\n\nmain {\n  let a = run A();\n  run B();\n  await Done == 2;\n\n  channel a.out (int, bool)> self.in;\n\n  await receive in(x,y);\n  print(\"Receive\", x, y);\n  print(\"DONE\");\n}"
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
    content: "shared {\n    let A = 1;\n    let B = 0;\n    let Start = false;\n}\nprogram A() {\n    await Start;\n\n    await receive in (x,y);\n\n    print(\"received\", x,y);\n\n    await first {\n        receive in (x,y) => {\n            print(\"first received \", x, y);\n        }\n        A == x => {\n            print(\"first A == x\");\n        }\n    }\n\n\n}\n\nmain {\n    let pa = run A();\n\n    channel self.out (int, bool)> pa.in;\n    channel self.out2 (int, bool)> pa.in;\n    Start = true;\n    send out (125, true);\n    send out2 (125, false);\n\n}"
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
    content: "main {\n    let n = 6;\n    let p: list(proc(A));\n    for i in 0..n {\n        let pid = run A(i);\n        p.push(pid);\n    }\n    \n    //not yet supported\n    //let p = [run A(i) for i in 0..10];\n\n    for i in 0..(n-1) {\n        let n = p.len();\n        let at_i = p.at(i);\n        let at_i2 = p.at((i+1)%n);\n        print(at_i, \"->\", at_i2);\n        channel at_i.out (int)> at_i2.in;\n    }\n\n    let first = p.at(0);\n    channel self.out (int)> first.in;\n    let n = p.len();\n    let last = p.at(n-1);\n    channel last.out (int)> self.in;\n\n    send out(0);\n\n    await receive in(i);\n    print(\"FINAL Received: \", i);\n}\n\n\nprogram A(id:int) {\n    print(\"Hello from A\");\n    await receive in (i);\n    id += i;\n    print(\"Received\", i, \" new value is \", id);\n    \n    send out(id);\n}"
  },
  {
    fileName: "test-reaches-simple.alt",
    title: "Test-Reaches-Simple",
    description: "Example program: Test-Reaches-Simple",
    tags: ["example","programs","conditionals"],
    content: "program A() {\n    label START;\n    let x = 1;\n    label MIDDLE;\n    let y = 2;\n}\n\nmain {\n    run A();\n}\n\ncheck {\n    always (if $.procs.A.at(0).reaches(MIDDLE) { eventually $.procs.A.at(0).reaches(end) });\n}"
  },
  {
    fileName: "test-wait.alt",
    title: "Test-Wait",
    description: "Example program: Test-Wait",
    tags: ["example","shared","concurrency","programs","loops","conditionals","synchronization","ring"],
    content: "shared {\n    let VA = 1;\n}\n\nmain {\n\n    print(\"await first\");\n    //print \"CASE 1\" (because the keyword first is used)\n    await first {\n        (VA == 0) => { print(\"CASE 0\"); }\n        (VA == 1) => { print(\"CASE 1\"); VA = 2; }\n        (VA == 2) => { print(\"CASE 2\"); }\n    }\n    \n    print(\"await seq\");\n    VA = 1;\n    //print \"CASE 1\" and \"CASE 2\"\n    await seq {\n        (VA == 0) => { print(\"CASE 0\"); }\n        (VA == 1) => { print(\"CASE 1\"); VA = 2; }\n        (VA == 2) => { print(\"CASE 2\"); }\n    }\n    \n    \n    if VA == 0 {\n        print(\"if condition\");\n    }\n\n    VA = 0; // comment to see a deadlock\n    \n    await (VA == 0);\n    print(\"await condition\");\n}\n/**\n`condition` is a boolean expression\n`await condition` is a statement that waits for the condition to be true\n\n```\nfirst { \n    condition1 => block1,\n    condition2 => block2,\n}\n``` \nis an boolean expression that is true if one of the conditions is true. Each condition is evaluated sequentially from top to bottom, if one condition is true, it executes only the first corresponding block and then goes to the first instruction outside the block, hence:\n```\nawait first {\n    condition1 => block1,\n    condition2 => block2,\n}\n```\nwaits for one of the conditions to be true, then executes only the corresponding block, then continues with the rest of the program\n\nSimilarly, \n```\nseq { \n    condition1 => block1,\n    condition2 => block2,\n}\n```\nis an boolean expression that evaluates to true if one of the conditions is true, however here, when the block corresponding to the first true condition is executed, the remaining conditions are also evaluated, and the blocks associated with all the true conditions are executed sequentially from top to bottom. Hence,\n```\nawait seq {\n    condition1 => block1,\n    condition2 => block2,\n}\n```\nwaits for one of the conditions to be true, then executes the corresponding block, then evaluate the remaining conditions from this first true condition and execute all the blocks associated with true conditions. Then, the rest of the program continues.\n\nSince seq {} and first {} are boolean expressions, they can be used in if statements, while loops, and first/seq conditions.\n\nExample:\n```\nif seq {\n        first {\n            condition1 => block1,\n            condition2 => block2,\n        }\n        condition3 => block3,\n    } \n{\n    print(\"if seq\")\n}\n```\nmeans that if condition1 is true, block1 is executed, then condition3 is evaluated (if it is true, block3 is executed). Otherwise if condition1 is false, then condition2 is evaluated, if it is true, block2 is executed, then condition3 is evaluated (if it is true, block3 is executed), if condition2 is false, condition3 is evaluated, if it is true, block3 is executed. If all conditions are false, the entier expression is false and the pring statement is not executed.\n */"
  },
  {
    fileName: "traffic_lights.alt",
    title: "Traffic Lights",
    description: "Example program: Traffic Lights",
    tags: ["example","shared","concurrency","programs","conditionals"],
    content: "shared {\n    // 0: Green, 1: Yellow, 2: Red\n    let Ns_state = 0;\n    let Ew_state = 2;\n}\n\nprogram TrafficController() {\n    loop {\n        // NS is Green, EW is Red\n        // Transition NS to Yellow\n        Ns_state = 1;\n        \n        // Transition NS to Red\n        Ns_state = 2;\n        \n        // Transition EW to Green\n        Ew_state = 0;\n        \n        // Transition EW to Yellow\n        Ew_state = 1;\n        \n        // Transition EW to Red\n        Ew_state = 2;\n        \n        // Transition NS to Green\n        Ns_state = 0;\n    }\n}\n\ncheck {\n    // Safety: Cannot have both lights passing (Green or Yellow) at the same time\n    // Passing means state < 2 (0 or 1)\n    always ( (Ns_state == 2) || (Ew_state == 2) );\n}\n\ncheck {\n    // Liveness: If NS is Red, it eventually becomes Green\n    always ( if (Ns_state == 2) { eventually (Ns_state == 0) } );\n}\n\ncheck {\n    // Liveness: If EW is Red, it eventually becomes Green\n    always ( if (Ew_state == 2) { eventually (Ew_state == 0) } );\n}\n\nmain {\n    run TrafficController();\n}"
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
