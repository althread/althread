---
sidebar_position: 1
---

# Introduction

## Qu'est ce qu'Althread ?

Althread est un langage de programmation pédagogique conçu pour modéliser et vérifier des systèmes concurrents et distribués. Inspiré du langage [PROMELA](https://fr.wikipedia.org/wiki/PROMELA), Althread offre une syntaxe simplifiée tout en préservant des fonctionnalités essentielles à la vérification de systèmes distribués comme la modélisation de processus parallèles, de communications entre ces processus et de comportements non déterministes.

:::info
Ce langage est particulièrement adapté à l'enseignement des bases de la programmation concurrente et à la vérification formelle, permettant aux étudiants et aux développeurs débutants de se familiariser avec ces concepts complexes dans un environnement accessible.
:::

## Objectifs d'Althread

Le développement d'althread est motivé par les objectifs suivants :
1. **Facilité d'apprentissage** : Althread est conçu pour être simple à apprendre et à utiliser, même pour des débutants en programmation. Sa syntaxe, inspirée du C, permet de le prendre en main rapidement et de se concentrer sur les concepts plutôt que sur la syntaxe.
2. **Accessibilité** : Althread est un langage open-source et multiplateforme, permettant à chacun de l'utiliser gratuitement et de contribuer à son développement. 
3. **Vérification de systèmes** : Althread permet de modéliser des systèmes concurrents et distribués et de vérifier leur validité en utilisant des conditions.
4. **Débogage** : Grâce à un outil de débogage intégré, les erreurs peuvent être rapidement identifiées puis corrigées, facilitant la correction de modèles complexes.

## Principes fondamentaux

| Fonctionnalité | Description                                                                                                                                   |
| -------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| Processus      | Althread permet de créer et d'exécuter plusieurs processus en parallèle de manière non déterministe.                                          |
| Communications | Les processus communiquent à travers des variables partagées ainsi que des canaux, permettant la synchronisation et l'échange d'informations. |
| Vérification   | Des conditions simples peuvent être définies pour vérifier la validité d'un système.                                                          |
| Débogage       | L'outil de débogage intégré aide à analyser les comportements inattendus et à identifier les erreurs de conception.                           |

## Exemple de code

Voici la modélisation de l'exclusion mutuelle de Dekker en Althread :

```althread

shared {
    const A_TURN = 1;
    const B_TURN = 2;
    let X: bool = false;
    let Y: bool = false;
    let T: int = 0;
    let NbSC = 0;
}

program A() {
    X = true;
    T = B_TURN;
    wait Y == false || T == A_TURN;

    NbSC += 1;
    //section critique
    NbSC -= 1;

    X = false;
}

program B() {
    Y = true;
    T = A_TURN;
    wait X == false || T == B_TURN;

    NbSC += 1;
    //section critique
    NbSC -= 1;

    Y = false;
}

always {
    NbSC == 0 || NbSC == 1;
}

main {
    run A();
    run B();
}
```