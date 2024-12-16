---
sidebar_position: 1
---

# Using Programs
We will now see how to create and execute programs in Althread. A program is simply an algorithm that, when executed, becomes a process, an independent execution unit that can run in parallel with other processes. Processes can communicate with each other using shared variables or channels.


## Program Declaration

To declare a program, you must use the `program` keyword. Here's an example of a program declaration:

```althread
program MyProgram() {
    // program code
}
```

:::note
You can declare as many programs as you want. All declared programs are stored in a list.
:::

:::warning
It is not possible to have two programs with the same name.
:::

## Program Execution

To execute a program, you must use the `run` function. Here's an example of program execution:

```althread
main {
    run MyProgram();
}
```


:::note
A program can be executed multiple times in parallel, creating several independent processes. For example, to execute the `MyProgram` program twice in parallel, you can write:
```althread
main {
    run MyProgram();
    run MyProgram();
}
```
:::


### What Happens When a Program is Executed?

Once a program is executed, it becomes a process. Process execution occurs by iteration. Each iteration corresponds to the execution of an [atomic instruction](/docs/guide/getting-started/syntaxe#expression-atomique) of a randomly chosen process among the currently running processes. When a process is executed, it can perform operations such as variable assignment, function calls, reading or writing to channels, etc...


## Complete Example

Here's a complete example of an Althread system that executes two processes in parallel, one running the Prog1 program and the other running the main program:

```althread
program Prog1() {
    print("program 1");
}
main {
    run Prog1();
    print("main");
}
```

In this example, the `Prog1` program is executed in parallel with the main program. Here's how this program executes:
1. The `Prog1` program and the main program are declared and stored in the list of programs.
2. The main program is started and its process is added to the list of running processes.
3. A process is randomly drawn from the running processes. Here, since there is only the main process, it is the one executed.
4. The main program executes the `run Prog1();` instruction, which adds a process running the `Prog1` program to the list of running processes.
5. A process is randomly drawn from the running processes. Here, the main process and the process running `Prog1` are running, so one of the two is randomly executed (either the `print("main");` instruction or the `print("program 1");` instruction).
6. When a process has finished its execution, it is removed from the list of running processes.
7. When all processes have finished their execution, the system stops.


:::note
There is no priority regarding the order of program declaration: all declared programs are stored in the program list before the main program execution. However, we will see that type checking of communication channels is performed in the order of program declarations. Thus, channels should only be used when their types are known, therefore after creating them (although in practice the order in which this occurs during execution is arbitrary).
:::