---
sidebar_position: 1
---

# Installation

## Utilisation dans un navigateur

Le plus simple pour commencer à utiliser Althread est d'utiliser l'éditeur en ligne disponible sur [althread.github.io/editor](https://althread.github.io/editor). Cela vous permettra de tester le langage sans avoir à installer quoi que ce soit sur votre machine.

L'éditeur accepte aussi du contenu injecté par l'URL. Vous pouvez partager un programme avec une URL du type `https://althread.github.io/editor?fileName=main.alt&data=${encodeURIComponent(contenu)}`.

Pour des contenus plus compacts ou plus robustes vis-a-vis des caracteres speciaux, vous pouvez aussi utiliser `data64` avec du base64url: `https://althread.github.io/editor?fileName=main.alt&data64=<contenu-base64url>`.

## Installation locale

Pour pouvoir utiliser Althread sur votre machine, vous devez installer le compilateur Althread. 

* Cloner le projet github : `git clone https://github.com/althread/althread.git`
* Exécuter le programme (cela va installer les dépendance et exécuter le programme) : `cargo run --release`
* Vous pouvez aussi compiler le programme avec `cargo build --release` et exécuter le programme avec `./target/release/althread-cli`

Les commandes disponibles sont les suivantes:

### Compile

```
./target/release/althread-cli compile file.alt
```

compile le programme `file.alt` et affiche les potentielles erreurs. En cas de succès, affiche l'arbre de syntaxe abstraite, et le code généré.

### Run
    
```
./target/release/althread-cli run file.alt
```
compile et exécute le programme `file.alt`. En cas de succès, affiche le résultat de l'exécution. Utiliser l'option `--debug` pour voir les lignes exécutées par les processus. Utiliser l'option `--verbose` pour voir l'évollution de l'état de chaque processus. Utiliser l'option `--seed <seed>` pour fixer la seed du générateur de nombres aléatoires.

### Random search

```
./target/release/althread-cli random-search file.alt
```
compile et exécute le programme `file.alt` un grand nombre de fois en utilsant des valeurs aléatoires différentes. En cas de violation d'un invariant, indique la seed qui a causé l'erreur.


### Check

```
./target/release/althread-cli check file.alt
```

compile le programme `file.alt`, génère le graphe des états accessibles du système et vérifie que les invariants sont respectés dans chacun des états.




