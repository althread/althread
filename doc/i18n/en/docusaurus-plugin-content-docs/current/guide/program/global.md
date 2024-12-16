---
sidebar_position: 3
---

# Shared Variables
Until now, variables declared in a program are local to that program. This means a program cannot access variables from other programs:
```althread
program Prog1() {
    // This will error
    print(x); // x does not exist in this process
}
main {
    let x = 0;
    run Prog1();
}
```
:::danger
The code above will return an error: the `Prog1` program cannot access the `x` variable declared in the main program.
:::

## Declaring Shared Variables
To allow multiple processes to access the same variable, you must declare it as a shared variable. A shared variable is a variable that can be read and modified by multiple processes. Here's how to declare a shared variable:

```althread
shared {
    let X: int;
    let Y = false;
    const A = 42;
}
```

:::warning
A shared variable's name must always start with an uppercase letter.
:::

:::tip
Declarations in the `shared` block work like classic declarations: they can be constant or mutable, have any type, and can be assigned a value.
Only declarations are possible in the `shared` block.
:::

## Executing Processes with Shared Variables
During execution, the `shared` block is executed in one go before the processes. Shared variables are thus accessible and modifiable by all processes.

```althread
shared {
    let X : int;
}
program Prog1() {
    X++;
    wait X == 2;
}
main {
    run Prog1();
    run Prog1();
}
```

:::note
In this example, both `Prog1` processes increment the `X` variable by 1. The first process then waits for `X` to be equal to 2 before continuing.
:::
