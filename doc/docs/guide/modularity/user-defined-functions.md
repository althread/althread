---
sidebar_position: 1
---

# Fonctions

Tout langage de programmation complet doit offrir un certain degré de modularité afin de favoriser la réutilisation du code, la clarté et la maintenance des programmes.
En plus de permettre l’écriture de programmes structurés, Althread autorise la définition de fonctions par l’utilisateur. 

## Déclaration d'une fonction

Les fonctions dans Althread suivent un ensemble précis de règles de syntaxe et de sémantique.

Une fonction doit être déclarée en commençant par le mot-clé `fn`, suivi du nom de la fonction, d’une liste d’arguments entre parenthèses — sous la forme `(identifiant: type, ...)` — ou vide s’il n’y a pas d’arguments, et enfin d’un type de retour.

Voici un exemple de déclaration de fonction :

```althread
fn max(a: int, b: int) -> int {
    if (a > b) {
        return a;
    }
    return b;
}
```
:::note
Il est possible de déclarer autant de fonctions que vous le souhaitez.
:::

Le type de retour d’une fonction peut être soit `void`, soit un type de données existant (comme `int`, `float`, `bool`, etc.).

:::note 
Pour des raisons de simplicité, les types de retour multiples ne sont pas autorisés (par exemple : `-> int | float | bool` est interdit).
:::

La valeur retournée par une fonction doit obligatoirement correspondre au type déclaré comme type de retour.

Si le type de retour est void, l’instruction `return` n’est pas nécessaire. Elle peut toutefois être utilisée seule (`return;`) pour quitter la fonction prématurément.

Une fonction doit obligatoirement retourner une valeur sur tous les chemins d’exécution, sauf si son type de retour est `void`.


## Appel d'une fonction

L’appel de fonctions en Althread suit une syntaxe familière, proche de celle des langages impératifs classiques. Une fonction peut être appelée en utilisant son nom, suivi d’une liste d’arguments entre parenthèses.

Voici un exemple d’appel de fonction dans un bloc main :

```althread
main {
    print("Max between 5 and 10 is: " + max(5, 10));
}
```

Dans cet exemple, la fonction `max` est appelée avec les arguments `5` et `10`. Sa valeur de retour est ensuite concaténée à une chaîne de caractères, puis affichée via la fonction prédéfinie `print`.

Lors de l’appel :
1. Les arguments sont évalués de gauche à droite.
2. Un nouveau contexte d’exécution est créé pour la fonction appelée.
3. À la fin de l’exécution ou lors d’un `return`, la fonction retourne sa valeur et le contexte est détruit.

## Comportement des fonctions
Lors de l’exécution d’un programme, les fonctions en Althread respectent les principes suivants :

**Passage des arguments par valeur :**
Les arguments d’une fonction sont transmis par copie. Ainsi, toute modification locale d’un paramètre n’a aucun effet sur la valeur d’origine.

**Récursivité autorisée :**
Une fonction peut s’appeler elle-même, directement ou indirectement. Les appels récursifs sont entièrement supportés, tant que la pile d’appel n’est pas dépassée. Ceci permet d'implémenter des algorithmes classiques comme la Tour de Hanoï:

```althread
fn hanoi(n: int, source: string, auxiliary: string, target: string) -> void {
    if n > 0 {
        hanoi(n - 1, source, target, auxiliary);
        print("Move disk " + n + " from " + source + " to " + target);
        hanoi(n - 1, auxiliary, source, target);
    }
}

main {
    let num_disks = 3;
    hanoi(num_disks, "A", "B", "C");
}
```

**Définitions uniques :**
Il est interdit de définir plusieurs fonctions portant le même nom. Une redéfinition lève une erreur à la compilation.

**Appel invalide interdit :**
L’appel à une fonction non définie déclenche une erreur. Toute fonction utilisée dans le programme doit avoir été définie au préalable.

