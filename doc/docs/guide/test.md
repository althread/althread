---
sidebar_position: 6
---

# Vérification et Tests

Althread permet de vérifier formellement le comportement de vos programmes. Il existe deux approches : les invariants simples et la logique temporelle linéaire (LTL).

## Invariants Simples

Le bloc `always` permet de définir des propriétés simples sur l'état global du programme, qui doivent toujours être vraies, dans tous les états accessibles.

Exemple :
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
Ici, le bloc `always` vérifie que la variable partagée `X` est toujours supérieure ou égale à 0. Il n'est pas possible d'accéder aux variables locales des processus.
:::

## Logique Temporelle (LTL)

Pour des propriétés plus complexes impliquant le temps et la causalité (ex: "si je fais une requête, j'obtiens toujours une réponse plus tard"), Althread propose le bloc `check`.

Le bloc `check` contient une formule LTL (Linear Temporal Logic).

### Opérateurs LTL

| Opérateur | Syntaxe Althread | Description |
|-----------|------------------|-------------|
| Toujours | `always ( P )` | P doit être vrai maintenant et pour tout le futur. |
| Éventuellement | `eventually ( P )` | P doit être vrai à un moment donné (maintenant ou plus tard). |
| Suivant | `next ( P )` | P doit être vrai à l'état suivant. |
| Jusqu'à | `( P ) until ( Q )` | P doit être vrai jusqu'à ce que Q soit vrai (Q doit arriver). |
| Implication | `if P { Q }` | Si P est vrai, alors Q doit être vrai. |

### Exemples de formules

**Sûreté (Safety) :** "Deux feux ne sont jamais verts en même temps"
```althread
check {
    always (Feu1_rouge || Feu2_rouge);
}
```

**Vivacité (Liveness) :** "Si le feu est rouge, il finira par devenir vert"
```althread
check {
    always ( if (Feu == ROUGE) { eventually (Feu == VERT) } );
}
```

**Réponse :** "Toute requête reçoit une réponse"
```althread
check {
    always ( if Requete { eventually Reponse } );
}
```

## Structure d'un projet de vérification

Il est recommandé de grouper vos propriétés dans plusieurs blocs `check` pour isoler les problèmes.

```althread
check {
    // Propriété critique
    always ( X > 0 );
}

check {
    // Propriété de fairness
    always ( if Requete { eventually Reponse } );
}
```

:::tip Fonction assert
Pour des vérifications impératives au sein du code des processus, consultez la documentation de la [fonction `assert()`](../api/built-in-functions.md).
:::