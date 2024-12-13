---
sidebar_position: 2
---

# Machine virtuelle Althread

Les instructions de la machine virtuelles sont:

- `Expression <expr>`: Évalue une expression et ajoute le résultat sur la pile.
- `Push <value>`: Ajoute une valeur sur la pile.
- `Unstack <n>`: Retire `n` valeurs de la pile.
- `GlobalReads <vector of global variables>`: Ajoute les valeurs des variables globales sur la pile.
- 


```rust
pub enum InstructionType {
    Empty,
    Expression(ExpressionControl),
    Push(Literal),
    Unstack(UnstackControl),
    GlobalReads(GlobalReadsControl),
    GlobalAssignment(GlobalAssignmentControl),
    LocalAssignment(LocalAssignmentControl),
    JumpIf(JumpIfControl),
    Jump(JumpControl),
    Break(BreakLoopControl),
    RunCall(RunCallControl),
    EndProgram,
    FnCall(FnCallControl),
    Declaration(DeclarationControl),
    ChannelPeek(String),
    ChannelPop(String),
    Destruct(usize),
    Exit,
    WaitStart(WaitStartControl),
    Wait(WaitControl),
    Send(SendControl),
    SendWaiting,
    Connect(ConnectionControl),
    AtomicStart,
    AtomicEnd,
}
```