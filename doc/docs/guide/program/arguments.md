---
sidebar_position: 2
---


# Arguments

Un programme peut recevoir des arguments. Ces arguments sont des valeurs passées au programme lors de son exécution. Les arguments sont utilisés pour personnaliser les processus exécutant un programme.

Voici un exemple de déclaration de programme utilisant un argument `id` :

```althread

program MyProgram(id: int) {
    print("Programme ", id);
    if id == 0 {
        print("Je suis le premier processus");
    }
}
main {
    run MyProgram(0);
    run MyProgram(1);
    run MyProgram(2);
}
```

Dans cet exemple, le programme `MyProgram` prend un argument `id` de type `int`. Lorsque le programme est exécuté, l'argument `id` est passé à chaque instance du programme. Chaque instance du programme, c'est-à-dire chaque processus, peut ensuite utiliser la valeur de l'argument `id` pour personnaliser son comportement.

:::note
Attention, dans l'exemple ci-dessus, une fois les processus exécutant `MyProgram` démarrés, l'ordre d'exécution est arbitraire. Il est possible que le processus avec l'argument `id` égal à 0 ne soit pas le premier à s'exécuter!
:::