---
sidebar_position: 3
---

# Syntaxe d'Althread

La syntaxe d'althread est faite pour être la plus intuitive possible. Elle est inspirée du langage C et du Rust, ce qui permet de la prendre en main rapidement et de se concentrer sur les concepts plutôt que sur la syntaxe. 

Quelques points important à retenir :
- Chaque ligne est terminée par un point-virgule `;` et les blocs de code sont délimités par des accolades `{}`.
- Les blocs de codes sont obligatoire après les structures de contrôle (`if`, `while`, etc...). Cependant, les parenthèses ne sont pas obligatoires.
- Les variables sont déclarées avec le mot-clé `let` ou `const` suivi du nom de la variable, du type et de la valeur optionnelle.
- Les commentaires sont délimités par `//` pour un commentaire sur une ligne et `/* */` pour un commentaire sur plusieurs lignes.

```althread
main {
    let x: int = 5;
    const y = 3.4; // y est de type float

    /* 
    La fonction print affiche
    tous les arguments passés en paramètre
    */
    print("Hello world! y=", y);
}
```

## Structure d'un projet

Un projet est structuré en plusieurs blocs, qui peuvent correspondre à 3 types d'éléments :
- **Déclaration de variables globales** : `shared { ... }`
- **Vérification de conditions** : `always { ... }`, `never { ... }` ou `eventually { ... }`
- **Définition de programme** : `program A() { ... }` ou `main { ... }`

:::note
Le bloc main est le progamme principal. Il est exécuté en premier et sert a exécuter les autres programmes.
:::

## Type de données

Les variables en althread peuvent prendre les types suivants :
- **Vide** : `void`
- **Booléen** : `bool`
- **Entier** : `int`
- **Flottant** : `float`
- **Chaîne de caractères** : `string`
- **Processus exécutant un programme `A`** : `proc(A)`
- **Tableau d'élement de type TYPE** : `list(TYPE)`


### Typage statique

Althread utilise un typage statique ce qui signifie que le type d'une variable est déterminé lorsqu'elle est déclarée et ne peut pas être modifié par la suite. Ainsi, le programme suivant provoquera une erreur :

```althread
let x: int = 5;
x = 3.4; // Erreur : x est de type int et ne peut pas prendre de valeur de type float.
```

### Typage implicite

```althread
let a: int = 5;   // x est de type int et prend la valeur 5.
let b: bool;      // x est de type bool et prend la valeur par défaut false.
let c = 3.4;      // x est de type float et prend la valeur 3.4.
let d;            // x est de type void et prend la valeur par défaut `null`.
```

## Convention de nommage des variables

En althread, les variables local à un programme commence obligatoirement par une minuscule et les variables globales par une majuscule.

```althread
shared {
    let G = 5; // Y est une variable globale
    // This will error
    let g = 5; // erreur
}
program A() {
    let l = 5; // x est une variable locale
    // This will error
    let L = 5; // erreur
}
```

## Structures de contrôle et portée des variables

Althread propose plusieurs structures de contrôle pour gérer le flux d'exécution d'un programme :
- **Condition** : `if condition { ... } else { ... }`
- **Boucle While** : `while condition { ... }`
- **Boucle For** : `for i in 0..10 { ... }`
- **Boucle infinie** : `loop { ... }`
- **Scope** : `{ ... }`

Les boucles peuvent être interrompues à l'aide de l'instruction `break` ou `continue`, qui permettent respectivement de sortir de la boucle ou de passer à l'itération suivante.

:::info
Les variables déclarées dans une structure de contrôle sont visibles uniquement à l'intérieur de cette structure. Cela permet de limiter la portée des variables et d'éviter les conflits de noms. 
:::


## Instructions bloquantes

En althread, la seul instruction bloquante est l'attente d'une condition avec l'instruction `wait`. Cette instruction permet de mettre en pause l'exécution d'un processus jusqu'à ce que la condition soit vérifiée.

```althread
program A() {
    wait X == 5;
    print("x est égal à 5");
}
```

La condition peut être une expression booléenne comme dans l'exemple précédent, mais elle peut aussi être une reception d'un message sur un canal avec l'instruction `receive`, qui peut être vue comme une expression booléenne valant `true` si un message est reçu et `false` sinon.

```althread
program A() {

    wait receive channel_name(x);

    print("message reçu");
    // x n'est pas dans le scope
}
```
Dans l'exemple précédent, `x` n'est pas dans le scope après l'instruction `wait` car l'instruction `receive` est suivie de manière optionnelle d'un bloc d'instruction, permettant d'utiliser les variables reçues.

```althread	
program A() {
    wait receive channel_name(x) => {
        print("message reçu, x=", x);
        // x est dans le scope
    }
}
```

L'instruction `wait` peut aussi être utilisée pour attendre une condition parmis plusieurs conditions en la faisant suivre de l'instruction `first` ou `all`.

```althread
program A() {
    wait first {
        receive channel_name1(x) => {
            print("message reçu, x=", x);
        }
        receive channel_name2(y) => {
            print("message reçu, y=", y);
        }
        X == 5 => {
            print("x est égal à 5");
        }
    }
}
```

Dans cette construction, une condition booléenne peut aussi être suivie d'un bloc d'instruction afin d'exécuter des instructions si la condition est vérifiée.


## Expression atomique

Une expression atomique est la plus petite unité d'exécution. En althread, il existe 6 types d'expressions atomiques :
- **Déclaration** : `let x = 5;`
- **Affectation** : `x = 5;`,  `x++;`, `x += 1`;
- **Opération arithmétique** : `x + y;`, `x - y;`, `x * y;`, `x / y;`, `x % y;`
- **Scope atomique**: `atomic { ... }`
- **Appel de fonction** : `print("Hello world");`, `wait x == 5;`
- **Exécution de processus** : `run A();`

:::note
Les expressions atomiques ne peuvent pas être interrompues par un autre processus. Cela signifie que pendant qu'un processus exécute une expression atomique, aucun autre processus ne peut prendre la main.
:::