---
sidebar_position: 5
---

# Créer des tests

Nous allons maintenant voir comment créer des tests pour vos processus. Ces tests servent à contrôler les comportements de vos processus et à vérifier qu'ils fonctionnent correctement.

## Blocs de test

En Althread, il existe 3 types de blocs de tests :
- `always`: vérifie qu'une condition est remplie à chaque itération
- `never`: vérifie qu'une condition n'est jamais remplie lors de l'exécution
- `eventually`: vérifie qu'une condition est remplie à un moment donné

Voici un exemple de l'utilisation de ces conditions :
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
Ici, le bloc `always` vérifie que la variable `X` est toujours inférieure à 1. Le test ne passera que si le processus de type `B` est exécuté avant le processus de type `A`.
:::

:::info
Il n'est pas possible d'utiliser le bloc de test pour des variables locales à un processus.
:::

## Fonction assert (non implémentée)

La fonction assert permet de vérifier si une condition est remplie. Si la condition n'est pas remplie, le test échoue et affiche un message d'erreur.

Voici un exemple de l'utilisation de la fonction assert :
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
    
    assert(X < 1, "X doit être inférieur à 1");
}
```

:::info
Il est possible d'utiliser `assert`sur des variables locales à un processus.
:::