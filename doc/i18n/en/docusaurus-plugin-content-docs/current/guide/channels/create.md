---
sidebar_position: 2
---

# Creating a Channel
A communication channel can be created between two *processes* to allow them to communicate. Channel creation is done using the `channel` keyword. Here's an example of channel declaration:

```althread
channel p1.out (string, int)> p2.in;
```

In this example, a channel named `out` is created on process `p1` to send messages of type `(string, int)` to the channel named `in` on process `p2`. Messages must strictly have the declared type. For now, channels can only be used in one direction (from `p1` to `p2`, indicated by the `>` chevron).

:::note
The `self` keyword refers to the current process and can be used to create a channel with another process.

```althread
channel self.out (string, int)> p2.in;
```
:::

## Sending Messages

A message is sent on a channel using the `send` instruction. Here's an example of sending a message:

```althread
program Prog1() {
    send out("Hello", 42);
}
```

In this example, the message `(Hello, 42)` is sent on the `out` channel of the current process. For this instruction to be valid, a program must have declared an `out` channel on at least one process of type `Prog1`. This ensures that message types are consistent.

Thus, for the previous example to work, the `out` channel declaration must be attached to a program of type `Prog1`:
The complete code is as follows:

```althread
main {
    let p1 = run Prog1();
    channel p1.out (string, int)> self.in;
}
program Prog1() {
    send out("Hello", 42);
}
```

:::note
Since compilation occurs from top to bottom, it is necessary to declare channels before using them to ensure correct type checking. However, the `main` program is always compiled first, so it is possible to move the `main` program declaration to the bottom of the file.
:::

Sending a message is an asynchronous operation, meaning the process sending the message continues its execution without waiting for the recipient process to receive the message.

## Receiving Messages
A message is received on a channel using the `receive` instruction.
This is a particular operation that must be preceded by the `wait` instruction to make it blocking.
Here's an example of message reception:

```althread
main {
    let p1 = run Prog1();
    channel p1.out (string, int)> self.in;
    // highlight-next-line
    wait receive in (x, y) => {
        print("Message received: ", x, y);
    }
}
program Prog1() {
    send out("Hello", 42);
}
```

We can see that the received values are stored in the variables `x` and `y` and can only be used in the instruction block following the `receive` instruction.
The type of the variables is automatically deduced from the channel type.