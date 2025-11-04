---
sidebar_position: 3
---

# Modules & Packages

Althread's module system extends beyond simple file imports to provide structured organization of code through modules and packages, including support for remote dependencies.

## Local Modules with `mod.alt`

When you have a directory with multiple related files, you can organize them into a module using a special `mod.alt` file. This file acts as the entry point for the module and defines what gets exported.

### Directory Structure

```
math/
├── mod.alt          # Module entry point
├── integers.alt     # Integer operations
├── floats.alt       # Float operations
└── constants.alt    # Mathematical constants
```

### Creating a Module

The `mod.alt` file imports and re-exports the module's components:

```althread
// math/mod.alt
import [
    integers,
    floats,
    constants
]

// Re-export specific functions or use them directly
// The module system automatically makes imported items available
```

### Importing Modules

Instead of importing individual files, you can import the entire module by its directory name:

```althread
// main.alt
import [
    math  // Imports math/mod.alt and its contents
]

main {
    // Access functions from the math module
    let result = math.add(5, 10);        // From integers.alt
    let pi_val = math.PI;                // From constants.alt
    let sqrt_val = math.sqrt(16.0);      // From floats.alt
}
```

## Remote Dependencies

Althread supports importing packages from remote repositories, particularly GitHub, using a package manager integrated into the Althread CLI.

### Project Initialization

First, create a new Althread project:

```bash
althread-cli init                    # Creates alt.toml in current directory
althread-cli init --name my-project  # Specify custom name
```

This creates an `alt.toml` file with basic project metadata:

```toml
[package]
name = "my-project"
version = "0.1.0"

[dependencies]
```

### Adding Remote Dependencies

Use the Althread CLI to add remote dependencies to your `alt.toml`:

```bash
althread-cli add github.com/lucianmocan/math-alt
```

This updates your `alt.toml` file but doesn't download the dependencies yet:

```toml
[package]
name = "my-project"
version = "0.1.0"

[dependencies]
"github.com/lucianmocan/math-alt" = "*"
```

### Installing Dependencies

After adding dependencies to `alt.toml`, install them:

```bash
althread-cli install           # Install all dependencies
althread-cli install --force   # Force reinstall even if already present
```

This downloads and caches the dependencies locally, making them available for import.

### Importing Remote Packages

You can import from remote packages using their full path or specific modules:

```althread
import [
    github.com/lucianmocan/math-alt/algebra/integers,  // Specific file
    github.com/lucianmocan/math-alt/algebra            // Module (if it has mod.alt)
]

main {
    // Using specific file import
    print(integers.add(1, 2));
    
    // Using module import (accessing the same functionality)
    print(algebra.add(1, 2));
}
```

### Managing Dependencies

Update dependencies to their latest versions:

```bash
althread-cli update                           # Update all dependencies
althread-cli update github.com/user/package   # Update specific dependency
```

Remove dependencies:

```bash
althread-cli remove github.com/lucianmocan/math-alt
```

### Namespace Resolution

In Althread, the namespace identifier is the last segment of the import path:

- `github.com/lucianmocan/math-alt/algebra/integers` → accessible as `integers`
- `github.com/lucianmocan/math-alt/algebra` → accessible as `algebra`

## Module System Benefits

### 1. Clean Organization
```althread
// Instead of importing many individual files
import [
    utils/math/integers,
    utils/math/floats,
    utils/math/constants,
    utils/math/geometry
]

// Import the entire math module
import [
    utils/math
]
```

### 2. Controlled Exports
The `mod.alt` file controls what gets exposed from the module, allowing for better encapsulation.

### 3. Versioning Support
Remote packages support semantic versioning through the `alt.toml` configuration.

### 4. Dependency Management
The CLI handles downloading, updating, and managing package versions automatically.

## Best Practices

### Local Module Organization
- Use `mod.alt` files to group related functionality
- Keep module interfaces clean and focused
- Use `@private` to hide internal implementation details

### Remote Dependencies
- Pin specific versions: `"1.2.3"` instead of `"*"`
- Use semantic versioning for compatibility
- Regularly update dependencies with `althread-cli update`

### Namespace Management
- Choose clear, descriptive module names
- Use aliasing when necessary to avoid conflicts
- Follow consistent naming conventions

## Complete Workflow Example

```bash
# 1. Initialize a new project
althread-cli init --name calculator-app

# 2. Add dependencies
althread-cli add github.com/lucianmocan/math-alt

# 3. Install dependencies
althread-cli install

# 4. Use in code
```

```althread
// main.alt
import [
    github.com/lucianmocan/math-alt/algebra,
    github.com/lucianmocan/math-alt/geometry as geo
]

main {
    let sum = algebra.add(5, 3);
    let area = geo.circle_area(10.0);
    
    print("Sum: " + sum);
    print("Circle area: " + area);
}
```

```bash
# 5. Update dependencies periodically
althread-cli update

# 6. Build and run
althread-cli run main.alt
```

The Althread module and package system provides powerful tools for organizing code at both local and ecosystem levels, making Althread projects more maintainable and enabling easy code sharing across the community.