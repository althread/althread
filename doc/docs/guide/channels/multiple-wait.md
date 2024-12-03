---
sidebar_pos: 1
---

# Attente multiple de messages

Il est possible d'attendre des messages de plusieurs canaux simultanément. Pour cela, il suffit d'utiliser l'instruction `wait` suivit du type d'attente `first` ou `seq` et d'utiliser un bloc avec les différentes conditions (à la manière d'un `match` en Rust).

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

`wait first` signifie qu'un seul bloc de code sera exécuté. Si plusieurs conditions sont vérifiées simultanément, une seule sera considérée, le bloc correspondant sera exécuté, puis le processus continuera son exécution après le bloc `wait`.

`wait seq` signifie que, lorsqu'une condition est vérifiée, le bloc correspondant est exécuté, puis les conditions suivantes sont évaluées dans l'ordre et chaque bloc correspondant à une condition vérifiée est exécuté, puis le processus continuera son exécution après le bloc `wait`.

