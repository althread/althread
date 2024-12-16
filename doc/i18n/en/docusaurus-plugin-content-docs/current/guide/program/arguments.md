---
sidebar_position: 2
---

# Arguments

A program can receive arguments. These arguments are values passed to the program during its execution. Arguments are used to customize the processes running a program.
Here's an example of a program declaration using an argument `id`:

```althread
program MyProgram(id: int) {
    print("Program ", id);
    if id == 0 {
        print("I am the first process");
    }
}
main {
    run MyProgram(0);
    run MyProgram(1);
    run MyProgram(2);
}
```

In this example, the `MyProgram` program takes an argument `id` of type `int`. When the program is executed, the `id` argument is passed to each instance of the program. Each instance of the program, that is, each process, can then use the `id` argument value to customize its behavior.

:::note
Be careful, in the example above, once the processes running `MyProgram` are started, the execution order is arbitrary. It is possible that the process with the `id` argument equal to 0 will not be the first to execute!
:::