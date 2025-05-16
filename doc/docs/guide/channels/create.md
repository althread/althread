---
sidebar_position: 2
---

# Création d'un canal

Un canal de communication peut être créé entre deux *processus* pour leur permettre de communiquer. La création d'un canal se fait en utilisant le mot-clé `channel`. Voici un exemple de déclaration de canal :

```althread
channel p1.out (string, int)> p2.in;
```

Dans cet exemple, un canal nommé `out` est créé sur le processus `p1` pour envoyer des messages de type `(string, int)` au canal nommé `in` sur le processus `p2`. Les messages devront obligatoirement avoir le type déclaré. Pour le moment les canaux ne peuvent pas être utilisés que dans une seule direction (de `p1` vers `p2`, indiqué par le chevron `>`).

:::note
le mot clé `self`fait référence au processus courant et peut être utiliser pour créer un canal avec un autre processus.
```althread
channel self.out (string, int)> p2.in;
```
::: 

## Envoi de messages

Un message est envoyé sur un canal en utilisant l'instruction `send`. Voici un exemple d'envoi de message :

```althread
prgram Prog1() {
    send out("Hello", 42);
}
```

Dans cet exemple, le message `(Hello, 42)` est envoyé sur le canal `out` du processus courant. Pour que cette instruction soit valide, il faut qu'un programme ait déclaré un canal `out` sur sur au moin un processus de type `Prog1`. Cela permet de s'assurer que les types des messages sont cohérents.

Ainsi, pour que l'exemple précédent fonctionne, il faut que la déclaration du cannal `out` soit attachée à un programme de type `Prog1` :
Le code complet est le suivant:

```althread
main {
    let p1 = run Prog1();
    channel p1.out (string, int)> self.in;
}
program Prog1() {
    send out("Hello", 42);
}
```

:::note
La compilation s'effectuant dans l'ordre de haut en bas, il est nécessaire de déclarer les canaux avant de les utiliser pour que la vérifications des types soit correcte. Cependant, le programme `main` est toujours compilé en premier, il est donc possible de déplacer la déclaration du programme `main` en bas du fichier.
::: 

L'envoie d'un message est une opération asynchrone, c'est-à-dire que le processus qui envoie le message et continue son exécution sans attendre que le processus destinataire ait reçu le message.


## Réception de messages

Un message est reçu sur un canal en utilisant l'instruction `receive`. 
C'est une opération particulière qui doit être précédée de l'instruction `await` afin de la rendre bloquante. 
Voici un exemple de réception de message :

```althread
main {
    let p1 = run Prog1();
    channel p1.out (string, int)> self.in;
    // highlight-next-line
    await receive in (x, y) => {
        print("Message reçu : ", x, y);
    }
}
program Prog1() {
    send out("Hello", 42);
}
```

On voit que les valeurs reçues sont stockées dans les variables `x` et `y` et ne peuvent être utilisées que dans le bloc d'instruction suivant l'instruction `receive`.
Le type des variables est automatiquement déduit du type du canal.