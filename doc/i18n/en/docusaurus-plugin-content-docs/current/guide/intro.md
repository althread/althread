---
sidebar_position: 1
---


# Introduction

## What is Althread?

Althread is an educational programming language designed to model and verify concurrent and distributed systems. Inspired by the language [PROMELA](https://en.wikipedia.org/wiki/PROMELA), Althread offers a simplified syntax while preserving essential features for verifying distributed systems, such as modeling parallel processes, inter-process communication, and non-deterministic behavior.

:::info
This language is particularly well-suited for teaching the fundamentals of concurrent programming and formal verification, enabling students and beginner developers to grasp these complex concepts in an accessible environment.
:::

## Objectives of Althread

The development of Althread is driven by the following goals:
1. **Ease of learning**: Althread is designed to be simple to learn and use, even for beginners in programming. Its syntax, inspired by C, allows for quick adoption and lets users focus on concepts rather than syntax.
2. **Accessibility**: Althread is an open-source, cross-platform language, enabling everyone to use it for free and contribute to its development.
3. **System verification**: Althread facilitates the modeling and validation of concurrent and distributed systems by using defined conditions.
4. **Debugging**: With its integrated debugging tool, errors can be quickly identified and corrected, making it easier to troubleshoot complex models.

## Core Principles

| Feature         | Description                                                                                                                           |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| **Processes**   | Althread allows the creation and execution of multiple processes in parallel with non-deterministic behavior.                         |
| **Communication** | Processes communicate via shared variables and channels, enabling synchronization and information exchange.                         |
| **Verification** | Simple conditions can be defined to verify the validity of a system.                                                                |
| **Debugging**   | The built-in debugging tool helps analyze unexpected behaviors and identify design errors.                                            |

## Example Code

Below is an example of modeling Dekker's mutual exclusion algorithm in Althread:

```althread
shared {
    const A_TURN = 1;
    const B_TURN = 2;
    let X: bool = false;
    let Y: bool = false;
    let T: int = 0;
    let NbSC = 0;
}

program A() {
    X = true;
    T = B_TURN;
    await Y == false || T == A_TURN;

    NbSC += 1;
    // critical section
    NbSC -= 1;

    X = false;
}

program B() {
    Y = true;
    T = A_TURN;
    await X == false || T == B_TURN;

    NbSC += 1;
    // critical section
    NbSC -= 1;

    Y = false;
}

always {
    NbSC == 0 || NbSC == 1;
}

main {
    run A();
    run B();
}
```

This example demonstrates the use of shared variables, process communication, and mutual exclusion in a distributed environment using Althread.
