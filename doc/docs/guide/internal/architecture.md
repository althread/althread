---
sidebar_position: 1
---

# Architecture

Althread est un langage statiquement typé qui est compilé en instructions pour la machine virtuelle Althread. Cette machine virtuelle est un programme qui exécute les instructions du programme Althread. Les instructions sont des opérations de bas niveau qui manipulent les données du programme, mais ne sont pas aussi bas niveau que les instructions d'une machine physique. La machine virtuelle Althread est conçue pour être facile à implémenter et à comprendre, mais elle reste assez performante pour exécuter des programmes de taille raisonnable.
L'execution sur la machine virtuelle Althread est similaire à l'execution d'un programme sur un ordinateur standard, avec des piles d'execution par processus, où sont stockées les variables locales, et une zone de mémoire partagé. La machine virtuelle est décrite en détail dans la section [Machine virtuelle](/docs/guide/internal/vm.md).

Pour être exécuté sur la machine virtuelle, un programme Althread doit être compilé en instructions. Le compilateur Althread est un programme qui prend un programme Althread en entrée et produit une structure de donnée qui est directement utilisée par la machine virtuelle (pour le moment, il n'est pas possible de stocker la version compiler d'un programme).
Le compilateur Althread est décrit en détail dans la section [Compilateur](/docs/guide/internal/compiler.md). Il faut noté que le compilateur n'effectue aucune optimisation, il se contente de traduire le programme en instructions.