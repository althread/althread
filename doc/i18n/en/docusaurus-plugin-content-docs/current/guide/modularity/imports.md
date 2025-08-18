---
sidebar_position: 2
---

# Imports

Imports in Althread are designed to be quite straightforward, maintaining simplicity and clarity.

## Basic Import Syntax

A single `import` block can be declared anywhere in your file. It contains a list of relative paths from the importing file to the target file, without the `.alt` extension (for importing modules organized with `mod.alt` files, see the [Modules & Packages](./packages-modules) guide):

```althread
import [
    math,
    cool/fib,
    display
]
```

Each item in the import list is a relative filepath. When importing from a subdirectory like `cool/fib`, the module becomes available under its filename (`fib` in this case).

## Accessing Imported Elements

Once imported, you access elements from modules using dot notation:

```althread
import [
    math,
    cool/fib,
    display
]

main {
    // Call a function from 'math'
    let result = math.max(5, 10);
    print(result);
    
    // Access shared variable from 'cool/fib'
    print(fib.N);
    
    // Modify shared variables
    fib.N = 15;
    
    // Call functions from modules
    let fibResult = fib.fibonacci_iterative_N();
    
    // Run programs from modules
    run display.Hello();
}
```

## Aliasing

If you have naming conflicts or prefer shorter names, you can use aliasing with the `as` keyword:

```althread
import [
    math,
    cool/fib as fibonacci,
    display as d
]

main {
    print(math.max(7, 3));
    print(fibonacci.N);
    run d.Hello();
}
```

## Privacy Control

Althread provides the `@private` directive to control access to module elements:

- Functions marked with `@private` cannot be accessed from importing files
- `program` blocks can also be marked as `@private`
- Multiple `main` blocks can coexist if marked as `@private`
- Shared variables are always importable and modifiable
- Conditions (always/never/eventually) are imported but read-only

```althread
// In math.alt
@private
fn internal_helper(x: int) -> int {
    return x * 2;
}

fn max(a: int, b: int) -> int {
    // This function is public and can be imported
    if a > b {
        return a;
    }
    return b;
}
```

```althread
// In main.alt
import [math]

main {
    print(math.max(5, 10));      // Works - public function
    // math.internal_helper(5);  // Error - private function
}
```

## Import Rules and Validation

Althread enforces several rules to maintain code quality:

1. **No duplicate imports**: Each module can only be imported once per file
2. **Circular import detection**: Althread checks for and prevents circular dependencies
3. **Path validation**: Import paths must exist and be valid relative paths
4. **Unique module names**: After aliasing, all imported modules must have unique names

## What Gets Imported

When you import a module, you get access to:

- **Public functions**: Functions without the `@private` directive
- **Public programs**: Program blocks without the `@private` directive  
- **Shared variables**: All shared variables (always importable and modifiable)
- **Conditions**: Always/never/eventually conditions (imported as read-only)

## Channel Imports

Channels declared in imported modules are handled through a special precompilation phase that scans all imports for channel declarations and adds them to the global compiler context. This ensures proper type inference across module boundaries.

## Error Reporting

When errors occur in imported files, Althread provides clear error messages that include:
- The filepath where the error occurred
- An error stack for improved debugging
- Context about which import chain led to the error

## Example: Complete Import Usage

Here's a comprehensive example showing various import features:

```althread
// main.alt
import [
    utils/math,
    algorithms/sorting as sort,
    display
]

main {
    // Use imported functions
    let maximum = math.max(15, 23);
    print("Maximum: " + maximum);
    
    // Access and modify shared variables
    print("Original value: " + sort.threshold);
    sort.threshold = 100;
    print("Updated value: " + sort.threshold);
    
    // Run imported programs
    run display.ShowWelcome();
    
    // Use imported sorting algorithm
    let numbers: list(int);
    numbers.push(64);
    numbers.push(34);
    numbers.push(25);
    numbers.push(12);
    numbers.push(22);
    numbers.push(11);
    numbers.push(90);
    sort.quickSort(numbers);
    print("Sorted array: " + numbers);
}
```

This import system provides a clean, predictable way to organize and share code across your Althread projects while maintaining clear boundaries and access controls.