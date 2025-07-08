---
sidebar_position: 1
---

# Built-in Functions

Althread provides several built-in functions for common operations.

**`print(...)` - Display**

Displays the arguments passed as parameters to the console.

**Signature:**
```althread
print(arg1, arg2, ..., argN) -> void
print(arg1 + arg2 + ... + argN) -> void
```

**Parameters:**
- Accepts a variable number of arguments of any type
- Arguments can be separated by commas `,` or concatenated with `+`
- With commas: arguments are separated by spaces in the output
- With the `+` operator: arguments are concatenated directly

**Example:**
```althread
main {
    let x = 42;
    let name = "Althread";
    
    // With commas (separated by spaces)
    print("Hello world!");                     // Displays: Hello world!
    print("x =", x);                          // Displays: x = 42
    print("Language:", name, "version", 1.0); // Displays: Language: Althread version 1.0
    
    // With the + operator (direct concatenation)
    print("x = " + x);                        // Displays: x = 42
    print("Language: " + name + " version " + 1.0); // Displays: Language: Althread version 1.0
}
```

---

**`assert(condition, message)` - Verification**

Verifies that a condition is true. If the condition is false, the program stops with an error message.

**Signature:**
```althread
assert(condition: bool, message: string) -> void
```

**Parameters:**
- `condition`: Boolean expression to verify
- `message`: Error message to display if the condition is false

**Example:**
```althread
shared {
    let X: int = 0;
}

program A() {
    X++;
}

program B() {
    X--;
}

main {
    atomic {
        run A();
        run B();
    }
    
    assert(X == 0, "X should be equal to 0");
    assert(X < 1, "X must be less than 1");
}
```

**Usage with local variables:**
```althread
program Calculator() {
    let result = 10 / 2;
    assert(result == 5, "Incorrect division");
    
    let list: list(int);
    list.push(1);
    list.push(2);
    assert(list.len() == 2, "The list should contain 2 elements");
}
```

:::tip Recommended usage
`assert()` is particularly useful for:
- Verifying system invariants
- Testing program behavior
- Validating conditions after complex operations
:::

:::warning Program termination
If an assertion fails, the program stops immediately and displays the error message. Use `assert()` only for critical verifications.
:::
