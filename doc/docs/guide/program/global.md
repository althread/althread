---
sidebar_position: 3
---


# Variables partagées

Jusqu'à présent, les variables déclarées dans un programme sont locales à ce programme. Cela signifie qu'un programme ne peut pas accéder aux variables des autres programme :

```althread
program Prog1() {
    // This will error
    print(x); // x n'existe pas dans ce processus
}

main {
    let x = 0;
    run Prog1();
}
```
:::danger
Le code ci-dessus renverra une erreur : le programme `Prog1` ne peut pas accéder à la variable `x` déclarée dans le programme principal.
:::


## Déclaration de variables partagées

Pour permettre à plusieurs processus d'accéder à une même variable, vous devez la déclarer comme une variable partagée. Une variable partagée est une variable qui peut être lue et modifiée par plusieurs processus. Voici comment déclarer une variable partagée :

```althread
shared {
    let X: int;
    let Y = false;
    const A = 42;
}
```

:::warning
Le nom d'une variable partagée commence obligatoirement par une majuscule.
:::

:::tip
Les déclaration du bloc `shared` fonctionnent comme les déclarations classiques : elles peuvent être constantes ou mutables, avoir n'importe quel type et l'on peut leur assigner une valeur. 
Il n'est possible de faire que des déclarations dans le bloc `shared`.
:::


## Exécution de processus avec des variables partagées

Lors de l'exécution, le bloc `shared` est exécuté d'une traite avant les processus. Les variables partagées sont ainsi accessibles et modifiables par tous les processus.

```althread
shared {
    let X : int;
}

program Prog1() {
    X++;
    wait X == 2;
}

main {
    run Prog1();
    run Prog1();
}
```

:::note
Dans cet exemple, les deux processus `Prog1` incrémentent la variable `X` de 1. Le premier processus attend ensuite que `X` soit égal à 2 avant de continuer.
:::