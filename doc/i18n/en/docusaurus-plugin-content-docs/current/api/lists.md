---
sidebar_position: 2
---

# Lists

:::note Current limitation
Currently Althread does not support list declaration with direct initialization: `let my_list: list(int) = [10, 20];`

Lists must be declared empty and then filled using `push()` methods.
:::

Lists in Althread have several built-in methods for manipulating their elements.

**`push(element)` - Add an element**

Adds an element to the end of the list.

**Signature:**
```althread
list.push(element: T) -> void
```

**Example:**
```althread
let my_list: list(int); // []
my_list.push(1); // [1]
my_list.push(3); // my_list becomes [1, 3]
```

---

**`len()` - Get size**

Returns the number of elements in the list.

**Signature:**
```althread
list.len() -> int
```

**Example:**
```althread
let my_list: list(int);
my_list.push(1);
my_list.push(3);
let size = my_list.len(); // size = 2
```

---

**`at(index)` - Access an element**

Returns the element at the specified index.

**Signature:**
```althread
list.at(index: int) -> T
```

**Example:**
```althread
let my_list: list(int);
my_list.push(1);
my_list.push(3);
print(my_list.at(1)); // displays: 3
```

**Errors:**
- Negative index
- Index greater than or equal to the list size

---

**`set(index, element)` - Modify an element**

Modifies the element at the specified index with a new value.

**Signature:**
```althread
list.set(index: int, element: T) -> void
```

**Example:**
```althread
let my_list: list(int);
my_list.push(1);
my_list.push(3); // my_list: [1, 3]
my_list.set(1, 5); // my_list becomes [1, 5]
print(my_list.at(1)); // displays: 5
```

**Errors:**
- Negative index
- Index greater than or equal to the list size
- Element type incompatible with the list type

---

**`remove(index)` - Remove an element**

Removes and returns the element at the specified index.

**Signature:**
```althread
list.remove(index: int) -> T
```

**Example:**
```althread
let my_list: list(int);
my_list.push(1);
my_list.push(3); // my_list: [1, 3]
let removed_element = my_list.remove(1); // removed_element = 3
// my_list becomes [1]
```

**Errors:**
- Negative index
- Index greater than or equal to the list size

## Complete usage example

```althread
main {
    let processes: list(proc(A));
    
    // Create and add processes
    for i in 0..3 {
        let p = run A(i);
        processes.push(p);
    }
    
    print("Number of processes:", processes.len());
    
    // Access processes
    for i in 0..processes.len() {
        let p = processes.at(i);
        print("Process at index", i, ":", p);
    }
    
    // Remove the last process
    if processes.len() > 0 {
        let last = processes.remove(processes.len() - 1);
        print("Removed process:", last);
    }
}
```
