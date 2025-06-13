---
sidebar_position: 1
---

# Exemples

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
    //section critique
    NbSC -= 1;

    X = false;
}

program B() {
    Y = true;
    T = A_TURN;
    await X == false || T == B_TURN;

    NbSC += 1;
    //section critique
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

