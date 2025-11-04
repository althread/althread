---
sidebar_position: 1
---

# Fonctions intégrées

Althread fournit plusieurs fonctions intégrées pour les opérations courantes.

**`print(...)` - Affichage**

Affiche les arguments passés en paramètres dans la console.

**Signature :**
```althread
print(arg1, arg2, ..., argN) -> void
print(arg1 + arg2 + ... + argN) -> void
```

**Paramètres :**
- Accepte un nombre variable d'arguments de tout type
- Les arguments peuvent être séparés par des virgules `,` ou concaténés avec `+`
- Avec les virgules : les arguments sont séparés par des espaces lors de l'affichage
- Avec l'opérateur `+` : les arguments sont concaténés directement

**Exemple :**
```althread
main {
    let x = 42;
    let nom = "Althread";
    
    // Avec des virgules (séparés par des espaces)
    print("Hello world!");                    // Affiche: Hello world!
    print("x =", x);                         // Affiche: x = 42
    print("Langage:", nom, "version", 1.0);  // Affiche: Langage: Althread version 1.0
    
    // Avec l'opérateur + (concaténation directe)
    print("x = " + x);                       // Affiche: x = 42
    print("Langage: " + nom + " version " + 1.0); // Affiche: Langage: Althread version 1.0
}
```

---

**`assert(condition, message)` - Vérification**

Vérifie qu'une condition est vraie. Si la condition est fausse, le programme s'arrête avec un message d'erreur.

**Signature :**
```althread
assert(condition: bool, message: string) -> void
```

**Paramètres :**
- `condition` : Expression booléenne à vérifier
- `message` : Message d'erreur à afficher si la condition est fausse

**Exemple :**
```althread
shared {
    let X: int = 0;
}

program A() {
    X++;
}

program B() {
    X--;
}

main {
    atomic {
        run A();
        run B();
    }
    
    assert(X == 0, "X devrait être égal à 0");
    assert(X < 1, "X doit être inférieur à 1");
}
```

**Utilisation avec variables locales :**
```althread
program Calculator() {
    let result = 10 / 2;
    assert(result == 5, "Division incorrecte");
    
    let liste: list(int);
    liste.push(1);
    liste.push(2);
    assert(liste.len() == 2, "La liste devrait contenir 2 éléments");
}
```

:::tip Usage recommandé
`assert()` est particulièrement utile pour :
- Vérifier les invariants de votre système
- Tester le comportement de vos programmes
- Valider les conditions après des opérations complexes
:::

:::warning Arrêt du programme
Si une assertion échoue, le programme s'arrête immédiatement et affiche le message d'erreur. Utilisez `assert()` pour les vérifications critiques uniquement.
:::
