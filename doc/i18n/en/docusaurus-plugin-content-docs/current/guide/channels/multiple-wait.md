---
sidebar_pos: 1
---

# Multiple Message Waiting
It is possible to wait for messages from multiple channels simultaneously. To do this, simply use the `await` instruction followed by the waiting type `first` or `seq` and use a block with different conditions (similar to a `match` in Rust).

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

`await first` means that only one block of code will be executed. If multiple conditions are verified simultaneously, only one will be considered, the corresponding block will be executed, and then the process will continue its execution after the `await` block.
`await seq` means that when a condition is verified, the corresponding block is executed, then the following conditions are evaluated in order, and each block corresponding to a verified condition is executed, after which the process will continue its execution after the `await` block.