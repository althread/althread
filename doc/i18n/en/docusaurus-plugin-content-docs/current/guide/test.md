---
sidebar_position: 6
---

# Verification and Tests

Althread allows you to formally verify the behavior of your programs. There are two approaches: simple invariants and Linear Temporal Logic (LTL).

## Simple Invariants

The `always` block allows you to define simple properties on the global state of the program, which must always be true, in all reachable states.

Example:
```althread
shared {
    let X: int = 0;
}

program A() {
    X = X + 1;
}

always {
    X >= 0;
}
```

:::info
Here, the `always` block verifies that the shared variable `X` is always greater than or equal to 0. It is not possible to access local variables of processes.
:::

## Linear Temporal Logic (LTL)

For more complex properties involving time and causality (e.g., "if I make a request, I always get a response later"), Althread offers the `check` block.

The `check` block contains an LTL (Linear Temporal Logic) formula.

### LTL Operators

| Operator | Althread Syntax | Description |
|-----------|------------------|-------------|
| Always | `always ( P )` | P must be true now and for the entire future. |
| Eventually | `eventually ( P )` | P must be true at some point (now or later). |
| Next | `next ( P )` | P must be true in the next state. |
| Until | `( P ) until ( Q )` | P must be true until Q becomes true (Q must happen). |
| Implication | `if P { Q }` | If P is true, then Q must be true. |

### Formula Examples

**Safety:** "Two lights are never green at the same time"
```althread
check {
    always (Light1_red || Light2_red);
}
```

**Liveness:** "If the light is red, it will eventually turn green"
```althread
check {
    always ( if (Light == RED) { eventually (Light == GREEN) } );
}
```

**Response:** "Every request receives a response"
```althread
check {
    always ( if Request { eventually Response } );
}
```

## Structure of a Verification Project

It is recommended to group your properties into multiple `check` blocks to isolate issues.

```althread
check {
    // Critical property
    always ( X > 0 );
}

check {
    // Fairness property
    always ( if Request { eventually Response } );
}
```

:::tip Assert Function
For imperative checks within process code, see the documentation for the [`assert()` function](../api/built-in-functions.md).
:::
