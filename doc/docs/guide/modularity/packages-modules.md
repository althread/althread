---
sidebar_position: 3
---

# Modules & Paquets

Le système de modules d'Althread va au‑delà des simples importations de fichiers : il permet d'organiser le code via des modules et paquets, avec prise en charge des dépendances distantes.

## Modules locaux avec `mod.alt`

Lorsque vous avez un dossier contenant plusieurs fichiers liés, vous pouvez l'organiser comme module en ajoutant un fichier spécial `mod.alt`. Ce fichier sert de point d'entrée du module et définit ce qui est exporté.

### Structure du dossier

```
math/
├── mod.alt          # Point d'entrée du module
├── integers.alt     # Opérations sur entiers
├── floats.alt       # Opérations sur flottants
└── constants.alt    # Constantes mathématiques
```

### Création d'un module

Le fichier `mod.alt` importe et réexporte les composants du module :

```althread
// math/mod.alt
import [
    integers,
    floats,
    constants
]

// Réexporter des fonctions spécifiques ou les utiliser directement
// Le système de modules rend automatiquement les éléments importés disponibles
```

### Importer un module

Plutôt que d'importer des fichiers individuels, vous pouvez importer tout le module via le nom du dossier :

```althread
// main.alt
import [
    math  // Importe math/mod.alt et son contenu
]

main {
    // Accéder aux fonctions du module math
    let result = math.add(5, 10);        // Depuis integers.alt
    let pi_val = math.PI;                // Depuis constants.alt
    let sqrt_val = math.sqrt(16.0);      // Depuis floats.alt
}
```

## Dépendances distantes

Althread supporte l'importation de paquets depuis des dépôts distants (notamment GitHub) via un gestionnaire de paquets intégré à l'outil CLI.

### Configuration du projet

Les dépendances distantes sont gérées dans un fichier `alt.toml` à la racine du projet :

```toml
[package]
name = "my-project"
version = "0.1.0"

[dependencies]
```

### Ajouter des dépendances distantes

Utilisez la CLI pour ajouter une dépendance au `alt.toml` :

```bash
althread-cli add github.com/lucianmocan/math-alt
```

Cette commande met à jour `alt.toml` mais ne télécharge pas encore les dépendances :

```toml
[package]
name = "my-project"
version = "0.1.0"

[dependencies]
"github.com/lucianmocan/math-alt" = "*"
```

### Installer les dépendances

Après avoir ajouté des dépendances dans `alt.toml`, installez‑les :

```bash
althread-cli install           # Installe toutes les dépendances
althread-cli install --force   # Réinstalle même si déjà présentes
```

Cela télécharge et met en cache les dépendances localement, les rendant availables pour l'import.

### Importer des paquets distants

Vous pouvez importer depuis des paquets distants en utilisant le chemin complet ou en ciblant un module :

```althread
import [
    github.com/lucianmocan/math-alt/algebra/integers,  // Fichier spécifique
    github.com/lucianmocan/math-alt/algebra            // Module (si il contient mod.alt)
]

main {
    // Import d'un fichier spécifique
    print(integers.add(1, 2));
    
    // Import du module (même fonctionnalité via le module)
    print(algebra.add(1, 2));
}
```

Remarque : `althread-cli add` ajoute la dépendance à `alt.toml`. Exécutez ensuite `althread-cli install` pour la télécharger et pouvoir l'importer.

### Gestion des dépendances

Mettre à jour les dépendances :

```bash
althread-cli update                           # Met à jour toutes les dépendances
althread-cli update github.com/user/package   # Met à jour une dépendance spécifique
```

Supprimer une dépendance :

```bash
althread-cli remove github.com/lucianmocan/math-alt
```

### Résolution des espaces de noms

Conformément à la convention de Go, l'identifiant de l'espace de noms correspond au dernier segment du chemin d'import :

- `github.com/lucianmocan/math-alt/algebra/integers` → accessible comme `integers`
- `github.com/lucianmocan/math-alt/algebra` → accessible comme `algebra`

## Avantages du système de modules

1. Organisation claire
```althread
// Au lieu d'importer de nombreux fichiers individuels
import [
    utils/math/integers,
    utils/math/floats,
    utils/math/constants,
    utils/math/geometry
]

// Importer le module complet
import [
    utils/math
]
```

2. Exports contrôlés  
Le fichier `mod.alt` contrôle ce qui est exposé depuis le module, offrant une meilleure encapsulation.

3. Support du versioning  
Les paquets distants prennent en charge le versioning (semver) via la configuration `alt.toml`.

4. Gestion des dépendances  
La CLI s'occupe du téléchargement, de la mise à jour et de la gestion des versions.

## Bonnes pratiques

### Organisation locale
- Utilisez `mod.alt` pour regrouper les fonctionnalités liées
- Maintenez des interfaces de module propres et focalisées
- Utilisez `@private` pour masquer l'implémentation interne

### Dépendances distantes
- Épinglez des versions précises (`"1.2.3"` plutôt que `"*"`)
- Suivez le versioning sémantique pour la compatibilité
- Mettez régulièrement à jour avec `althread-cli update`

### Gestion des noms
- Choisissez des noms de module clairs et explicites
- Utilisez des alias si nécessaire pour éviter les conflits
- Adoptez des conventions de nommage cohérentes

## Exemple : workflow complet

```bash
# 1. Initialiser un nouveau projet
althread-cli init --name calculator-app

# 2. Ajouter des dépendances
althread-cli add github.com/lucianmocan/math-alt

# 3. Installer les dépendances
althread-cli install

# 4. Utiliser dans le code
```

```althread
// main.alt
import [
    github.com/lucianmocan/math-alt/algebra,
    github.com/lucianmocan/math-alt/geometry as geo
]

main {
    let sum = algebra.add(5, 3);
    let area = geo.circle_area(10.0);
    
    print("Sum: " + sum);
    print("Circle area: " + area);
}
```

```bash
# 5. Mettre à jour périodiquement les dépendances
althread-cli update

# 6. Construire et lancer
althread-cli run main.alt
```

Le système de modules et paquets d'Althread fournit des outils puissants pour organiser le code localement et au sein de l'écosystème, rendant les projets plus maintenables et facilitant le partage et la réutilisation du code.