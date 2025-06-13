---
sidebar_position: 3
---

# Althread Syntax

Althread's syntax is designed to be as intuitive as possible. It is inspired by C and Rust, which allows for quick adoption and focuses on concepts rather than syntax.

Some important points to remember:
- Each line ends with a semicolon `;` and code blocks are delimited by curly braces `{}`.
- Code blocks are mandatory after control structures (`if`, `while`, etc...). However, parentheses are not required.
- Variables are declared with the `let` or `const` keyword followed by the variable name, type, and optional value.
- Comments are delimited by `//` for single-line comments and `/* */` for multi-line comments.

```althread
main {
    let x: int = 5;
    const y = 3.4; // y is of type float

    /* 
    The print function displays
    all arguments passed as parameters
    */
    print("Hello world! y=", y);
}
```

## Project Structure

A project is structured in several blocks, which can correspond to 3 types of elements:
- **Global variable declaration**: `shared { ... }`
- **Condition verification**: `always { ... }`, `never { ... }` or `eventually { ... }`
- **Program definition**: `program A() { ... }` or `main { ... }`

:::note
The main block is the main program. It is executed first and is used to run other programs.
:::

## Data Types

Variables in Althread can have the following types:
- **Void**: `void`
- **Boolean**: `bool`
- **Integer**: `int`
- **Float**: `float`
- **String**: `string`
- **Process running program `A`**: `proc(A)`
- **Array of elements of type TYPE**: `list(TYPE)`

### Static Typing

Althread uses static typing, which means the type of a variable is determined when it is declared and cannot be modified afterwards. Thus, the following program will cause an error:

```althread
let x: int = 5;
x = 3.4; // Error: x is of type int and cannot take a float value.
```

### Implicit Typing

```althread
let a: int = 5;   // x is of type int and takes the value 5.
let b: bool;      // x is of type bool and takes the default value false.
let c = 3.4;      // x is of type float and takes the value 3.4.
let d;            // x is of type void and takes the default value `null`.
```

## Variable Naming Convention

In Althread, local variables to a program must start with a lowercase letter, and global variables with an uppercase letter.

```althread
shared {
    let G = 5; // G is a global variable
    // This will error
    let g = 5; // error
}
program A() {
    let l = 5; // l is a local variable
    // This will error
    let L = 5; // error
}
```

## Control Structures and Variable Scope

Althread offers several control structures to manage a program's execution flow:
- **Condition**: `if condition { ... } else { ... }`
- **While Loop**: `while condition { ... }`
- **For Loop**: `for i in 0..10 { ... }`
- **Infinite Loop**: `loop { ... }`
- **Scope**: `{ ... }`

Loops can be interrupted using the `break` or `continue` instructions, which allow exiting the loop or moving to the next iteration respectively.

:::info
Variables declared in a control structure are only visible inside that structure. This helps limit variable scope and avoid name conflicts.
:::

## Blocking Instructions

In Althread, the only blocking instruction is waiting for a condition with the `await` instruction. This instruction allows pausing a process's execution until the condition is verified.

```althread
program A() {
    await X == 5;
    print("x is equal to 5");
}
```

The condition can be a boolean expression as in the previous example, but it can also be receiving a message on a channel with the `receive` instruction, which can be seen as a boolean expression that is `true` if a message is received and `false` otherwise.

```althread
program A() {
    await receive channel_name(x);

    print("message received");
    // x is not in scope
}
```
In the previous example, `x` is not in scope after the `await` instruction because the `receive` instruction is optionally followed by an instruction block, allowing the use of received variables.

```althread	
program A() {
    await receive channel_name(x) => {
        print("message received, x=", x);
        // x is in scope
    }
}
```

The `await` instruction can also be used to wait for a condition among multiple conditions by following it with the `first` or `all` instruction.

```althread
program A() {
    await first {
        receive channel_name1(x) => {
            print("message received, x=", x);
        }
        receive channel_name2(y) => {
            print("message received, y=", y);
        }
        X == 5 => {
            print("x is equal to 5");
        }
    }
}
```

In this construction, a boolean condition can also be followed by an instruction block to execute instructions if the condition is verified.

## Atomic Expression

An atomic expression is the smallest unit of execution. In Althread, there are 6 types of atomic expressions:
- **Declaration**: `let x = 5;`
- **Assignment**: `x = 5;`, `x++;`, `x += 1`;
- **Arithmetic Operation**: `x + y;`, `x - y;`, `x * y;`, `x / y;`, `x % y;`
- **Atomic Scope**: `atomic { ... }`
- **Function Call**: `print("Hello world");`, `await x == 5;`
- **Process Execution**: `run A();`

:::note
Atomic expressions cannot be interrupted by another process. This means that while a process is executing an atomic expression, no other process can take control.
:::