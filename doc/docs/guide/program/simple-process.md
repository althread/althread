---
sidebar_position: 1
---


# Utilisation des programmes

Nous allons maintenant voir comment créer et exécuter des programmes en Althread. Un programme est simplement un algorithme qui en s'exécutant devient un processus, une unité d'exécution indépendante, qui peut s'exécuter en parallèle d'autres processus. Les processus peuvent communiquer entre eux en utilisant des variables partagées ou des canaux.

## Déclaration d'un programme

Pour déclarer un programme, vous devez utiliser le mot-clé `program`. Voici un exemple de déclaration de programme :

```althread
program MyProgram() {
    // code du programme
}
```

:::note
Il est possible de déclarer autant de programmes que vous le souhaitez. Tous les programmes déclarés sont stockés dans une liste
:::
:::warning
Il n'est pas possible d'avoir deux programmes avec le même nom.
:::

## Exécution d'un programme

Pour exécuter un programme, vous devez utiliser la fonction `run`. Voici un exemple d'exécution d'un programme :

```althread
main {
    run MyProgram();
}
```

:::note
Un programme peut être exécuté plusieurs fois en parallèle, ce qui crée plusieurs processus indépendants. Par exemple, pour exécuter deux fois le programme `MyProgram` en parallèle, vous pouvez écrire :

```althread
main {
    run MyProgram();
    run MyProgram();
}
```
:::

### Que se passe-t-il lorsqu'un programme est exécuté ?

Une fois un programme exécuté, il devient un processus. L'exécution d'un processus se fait par itération. Chaque itération correspond à l'exécution d'une [instruction atomique](/docs/guide/getting-started/syntaxe#expression-atomique) d'un processus choisi aléatoirement parmi les processus en cours d'exécution. Lorsqu'un processus est exécuté, il peut effectuer des opérations telles que l'assignation de variables, l'appel de fonctions, la lecture ou l'écriture de canaux, etc...

## Exemple complet

Voici un exemple complet d'un système Althread qui exécute deux processus en parallèle, l'un exécutant le programme Prog1 et l'autre le programme main:

```althread
program Prog1() {
    print("program 1");
}

main {
    run Prog1();
    print("main");
}
```

Dans cet exemple, le programme `Prog1` est exécuté en parallèle du programme principal. Voici comment s'exécute ce programme :
1. Le programme `Prog1` et le programme principal sont déclarés et stockés dans la liste des programmes.
2. Le programme principal est démarré et son processus est ajouté à la liste des processus en cours d'exécution.
3. Un processus est tiré aléatoirement parmi les processus en cours d'exécution. Ici, comme il n'y a que le processus principal, c'est lui qui est exécuté.
4. Le programme principal exécute l'instruction `run Prog1();`, ce qui ajoute un processus exécutant le programme `Prog1` à la liste des processus en cours d'exécution.
5. Un processus est tiré aléatoirement parmi les processus en cours d'exécution. Ici, le processus principal et le processus exécutant `Prog1` sont en cours d'exécution, donc l'un des deux est exécuté aléatoirement (soit l'instruction `print("main");`, soit l'instruction `print("program 1");`).
6. Quand un processus a terminé son exécution, il est retiré de la liste des processus en cours d'exécution.
7. Quand tous les processus ont terminé leur exécution, le système s'arrête.


:::note
Il n'y a pas de priorité quant à l'ordre de déclaration des programmes : tous les programmes déclarés sont stockés dans la liste des programmes avant l'exécution du programme principal. Cependant, on verra que la vérification des types des canaux de communication est effectuée dans l'ordre de déclaration des programmes. Ainsi, il faut utiliser les canaux uniquement quand leur types sont connus, donc après les avoir créés (même si en pratique l'ordre dans lequel cela intervient durant l'exécution est arbitraire).
:::
