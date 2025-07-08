---
sidebar_position: 6
---

# Creating Tests

Now, let’s look at how to create tests for your processes. These tests are used to monitor the behavior of your processes and ensure they function correctly.

## Test Blocks

In Althread, there are three types of test blocks:
- `always`: checks that a condition is met in every iteration.
- `never`: checks that a condition is never met during execution.
- `eventually`: checks that a condition is met at some point.

Here is an example of how these conditions can be used:

```althread
shared {
    let X: int;
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
}

always {
    X < 1;
}
```

:::note
In this example, the `always` block ensures that the variable `X` is always less than 1. The test will pass only if the `B` process is executed before the `A` process.
:::

:::info
It is not possible to use test blocks for variables local to a process.
:::

:::tip Assert function
For more flexible checks, see the documentation of the [`assert()` function`](../api/fonctions-intégrées.md#assert-vérification) which allows testing conditions with custom error messages.
:::
