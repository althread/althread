---
sidebar_position: 2
---

# Imports

Les imports dans Althread sont conçus pour être simples et clairs, tout en
restant faciles à utiliser.

## Syntaxe de base des imports

Un seul bloc `import` peut être déclaré n’importe où dans votre fichier.  
Il contient une liste de chemins relatifs depuis le fichier importeur vers le
fichier cible, sans l’extension `.alt` (pour l’importation de modules organisés
avec des fichiers `mod.alt`, voir le guide
[Modules & Packages](./packages-modules)) :

```althread
import [
    math,
    cool/fib,
    display
]
```

Chaque élément de la liste d’import est un chemin relatif. Lorsqu’on importe
depuis un sous-dossier comme `cool/fib`, le module devient accessible sous son
nom de fichier (`fib` dans ce cas).

## Accéder aux éléments importés

Une fois importés, vous accédez aux éléments des modules en utilisant la
notation par points :

```althread
import [
    math,
    cool/fib,
    display
]

main {
    // Appeler une fonction de 'math'
    let result = math.max(5, 10);
    print(result);
    
    // Accéder à une variable partagée de 'cool/fib'
    print(fib.N);
    
    // Modifier des variables partagées
    fib.N = 15;
    
    // Appeler des fonctions de modules
    let fibResult = fib.fibonacci_iterative_N();
    
    // Exécuter des programmes de modules
    run display.Hello();
}
```

## Alias

En cas de conflits de noms ou si vous préférez des noms plus courts, vous pouvez
utiliser des alias avec le mot-clé `as` :

```althread
import [
    math,
    cool/fib as fibonacci,
    display as d
]

main {
    print(math.max(7, 3));
    print(fibonacci.N);
    run d.Hello();
}
```

## Contrôle de la confidentialité

Althread fournit la directive `@private` pour contrôler l’accès aux éléments
d’un module :

- Les fonctions marquées avec `@private` ne peuvent pas être appelées depuis les
  fichiers importeurs
- Les blocs `program` peuvent aussi être marqués comme `@private`
- Plusieurs blocs `main` peuvent coexister s’ils sont marqués `@private`
- Les variables partagées sont toujours importables et modifiables
- Les conditions (always/never/eventually) sont importées mais en lecture seule

```althread
// Dans math.alt
@private
fn internal_helper(x: int) -> int {
    return x * 2;
}

fn max(a: int, b: int) -> int {
    // Cette fonction est publique et peut être importée
    if a > b {
        return a;
    }
    return b;
}
```

```althread
// Dans main.alt
import [math]

main {
    print(math.max(5, 10));      // OK - fonction publique
    // math.internal_helper(5);  // Erreur - fonction privée
}
```

## Règles et validation des imports

Althread applique plusieurs règles pour maintenir la qualité du code :

1. **Pas de doublons** : chaque module ne peut être importé qu’une seule fois par
   fichier
2. **Détection des imports circulaires** : Althread vérifie et empêche les
   dépendances circulaires
3. **Validation des chemins** : les chemins d’import doivent exister et être
   valides relativement au fichier
4. **Noms uniques** : après aliasing, tous les modules importés doivent avoir
   des noms uniques

## Ce qui est importé

Lorsqu’un module est importé, vous avez accès à :

- **Fonctions publiques** : celles sans directive `@private`
- **Programmes publics** : blocs `program` sans directive `@private`
- **Variables partagées** : toujours importables et modifiables
- **Conditions** : always/never/eventually (importées en lecture seule)

## Imports de canaux

Les canaux déclarés dans les modules importés sont traités via une phase
spéciale de précompilation qui scanne tous les imports pour détecter les
déclarations de canaux et les ajoute au contexte global du compilateur.  
Cela garantit une bonne inférence de types entre modules.

## Gestion des erreurs

Lorsqu’une erreur survient dans un fichier importé, Althread fournit des
messages clairs incluant :

- Le chemin du fichier où l’erreur s’est produite
- Une pile d’erreurs pour faciliter le débogage
- Le contexte de la chaîne d’imports ayant mené à l’erreur

## Exemple : utilisation complète des imports

Voici un exemple complet montrant différentes fonctionnalités d’import :

```althread
// main.alt
import [
    utils/math,
    algorithms/sorting as sort,
    display
]

main {
    // Utiliser des fonctions importées
    let maximum = math.max(15, 23);
    print("Maximum: " + maximum);
    
    // Accéder et modifier des variables partagées
    print("Valeur originale: " + sort.threshold);
    sort.threshold = 100;
    print("Valeur mise à jour: " + sort.threshold);
    
    // Exécuter des programmes importés
    run display.ShowWelcome();
    
    // Utiliser un algorithme de tri importé
    let numbers: list(int);
    numbers.push(64);
    numbers.push(34);
    numbers.push(25);
    numbers.push(12);
    numbers.push(22);
    numbers.push(11);
    numbers.push(90);
    sort.quickSort(numbers);
    print("Tableau trié: " + numbers);
}
```

Ce système d’import offre une manière claire et prévisible d’organiser et de
partager du code dans vos projets Althread, tout en maintenant des frontières
nettes et un bon contrôle des accès.