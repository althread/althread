---
sidebar_position: 1
---

# Functions

Any complete programming language must offer a certain degree of modularity to promote code reusability, clarity, and program maintenance.
In addition to allowing structured program writing, Althread allows user-defined functions.

## Function Declaration

Functions in Althread follow a precise set of syntax and semantic rules.

A function must be declared starting with the `fn` keyword, followed by the function name, a list of arguments in parentheses — in the form `(identifier: type, ...)` — or empty if there are no arguments, and finally a return type.

Here's an example of function declaration:

```althread
fn max(a: int, b: int) -> int {
    if (a > b) {
        return a;
    }
    return b;
}
```
:::note
You can declare as many functions as you want.
:::

A function's return type can be either `void` or an existing data type (like `int`, `float`, `bool`, etc.).

:::note 
For simplicity reasons, multiple return types are not allowed (for example: `-> int | float | bool` is forbidden).
:::

The value returned by a function must match the type declared as the return type.

If the return type is void, the `return` statement is not necessary. However, it can be used alone (`return;`) to exit the function prematurely.

A function must return a value on all execution paths, except if its return type is `void`.

## Function Call

Function calls in Althread follow familiar syntax, similar to classical imperative languages. A function can be called using its name, followed by a list of arguments in parentheses.

Here's an example of function call in a main block:

```althread
main {
    print("Max between 5 and 10 is: " + max(5, 10));
}
```

In this example, the `max` function is called with arguments `5` and `10`. Its return value is then concatenated to a string and displayed via the predefined `print` function.

During the call:
1. Arguments are evaluated from left to right.
2. A new execution context is created for the called function.
3. At the end of execution or during a `return`, the function returns its value and the context is destroyed.

## Function Behavior
During program execution, functions in Althread follow these principles:

**Pass by value:**
Function arguments are passed by copy. Thus, any local modification of a parameter has no effect on the original value.

**Recursion allowed:**
A function can call itself, directly or indirectly. Recursive calls are fully supported, as long as the call stack is not exceeded. This allows implementing classic algorithms like the Tower of Hanoi:

```althread
fn hanoi(n: int, source: string, auxiliary: string, target: string) -> void {
    if n > 0 {
        hanoi(n - 1, source, target, auxiliary);
        print("Move disk " + n + " from " + source + " to " + target);
        hanoi(n - 1, auxiliary, source, target);
    }
}

main {
    let num_disks = 3;
    hanoi(num_disks, "A", "B", "C");
}
```

**Unique definitions:**
It is forbidden to define multiple functions with the same name. Redefinition raises a compilation error.

**Invalid calls forbidden:**
Calling an undefined function triggers an error. Any function used in the program must have been defined beforehand.