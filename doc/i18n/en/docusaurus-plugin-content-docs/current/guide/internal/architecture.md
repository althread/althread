---
sidebar_position: 1
---

# Architecture
Althread is a statically typed language that is compiled into instructions for the Althread virtual machine. This virtual machine is a program that executes Althread program instructions. These instructions are low-level operations that manipulate program data, but are not as low-level as instructions on a physical machine. The Althread virtual machine is designed to be easy to implement and understand, while remaining sufficiently performant to execute programs of a reasonable size.

Execution on the Althread virtual machine is similar to running a program on a standard computer, with execution stacks per process, where local variables are stored, and a shared memory area. The virtual machine is described in detail in the [Virtual Machine](/docs/guide/internal/vm.md) section.

To be executed on the virtual machine, an Althread program must be compiled into instructions. The Althread compiler is a program that takes an Althread program as input and produces a data structure that is directly used by the virtual machine (at the moment, it is not possible to store the compiled version of a program).

The Althread compiler is described in detail in the [Compiler](/docs/guide/internal/compiler.md) section. It should be noted that the compiler performs no optimization; it simply translates the program into instructions.