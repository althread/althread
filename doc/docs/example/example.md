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

## Feux de Signalisation (Vérification LTL)

Cet exemple modélise un contrôleur de feux de signalisation et utilise la vérification LTL pour s'assurer que les feux ne sont jamais verts simultanément (sécurité) et qu'ils finissent toujours par changer (vivacité).

```althread
shared {
    // 0: Vert, 1: Jaune, 2: Rouge
    let Ns_state = 0;
    let Ew_state = 2;
}

program TrafficController() {
    loop {
        // NS est Vert, EW est Rouge
        // Transition NS vers Jaune
        Ns_state = 1;

        // Transition NS vers Rouge
        Ns_state = 2;

        // Transition EW vers Vert
        Ew_state = 0;

        // Transition EW vers Jaune
        Ew_state = 1;

        // Transition EW vers Rouge
        Ew_state = 2;

        // Transition NS vers Vert
        Ns_state = 0;
    }
}

check {
    // Sécurité : Les deux feux ne doivent pas laisser passer (Vert ou Jaune) en même temps.
    // "Laisser passer" signifie état < 2 (0 ou 1)
    always ( (Ns_state == 2) || (Ew_state == 2) );
}

check {
    // Vivacité : Si le feu NS est rouge, il finira par devenir vert
    always ( if (Ns_state == 2) { eventually (Ns_state == 0) } );
}

check {
    // Vivacité : Si le feu EW est rouge, il finira par devenir vert
    always ( if (Ew_state == 2) { eventually (Ew_state == 0) } );
}

main {
    run TrafficController();
}
```

