---
sidebar_position: 2
---

# Listes

:::note Limitation actuelle
Actuellement Althread ne supporte pas la déclaration des listes avec initialisation directe : `let ma_liste: list(int) = [10, 20];`

Les listes doivent être déclarées vides puis remplies avec les méthodes `push()`.
:::

Les listes en Althread disposent de plusieurs méthodes intégrées pour manipuler leurs éléments.

**`push(element)` - Ajouter un élément**

Ajoute un élément à la fin de la liste.

**Signature :**
```althread
list.push(element: T) -> void
```

**Exemple :**
```althread
let ma_liste: list(int); // []
ma_liste.push(1); // [1]
ma_liste.push(3); // ma_liste devient [1, 3]
```

---

**`len()` - Obtenir la taille**

Retourne le nombre d'éléments dans la liste.

**Signature :**
```althread
list.len() -> int
```

**Exemple :**
```althread
let ma_liste: list(int);
ma_liste.push(1);
ma_liste.push(3);
let taille = ma_liste.len(); // taille = 2
```

---

**`at(index)` - Accéder à un élément**

Retourne l'élément à l'index spécifié.

**Signature :**
```althread
list.at(index: int) -> T
```

**Exemple :**
```althread
let ma_liste: list(int);
ma_liste.push(1);
ma_liste.push(3);
print(ma_liste.at(1)); // affiche: 3
```

**Erreurs :**
- Index négatif
- Index supérieur ou égal à la taille de la liste

---

**`set(index, element)` - Modifier un élément**

Modifie l'élément à l'index spécifié avec une nouvelle valeur.

**Signature :**
```althread
list.set(index: int, element: T) -> void
```

**Exemple :**
```althread
let ma_liste: list(int);
ma_liste.push(1);
ma_liste.push(3); // ma_liste : [1, 3]
ma_liste.set(1, 5); // ma_liste devient [1, 5]
print(ma_liste.at(1)); // affiche: 5
```

**Erreurs :**
- Index négatif
- Index supérieur ou égal à la taille de la liste
- Type de l'élément incompatible avec le type de la liste

---

**`remove(index)` - Supprimer un élément**

Supprime et retourne l'élément à l'index spécifié.

**Signature :**
```althread
list.remove(index: int) -> T
```

**Exemple :**
```althread
let ma_liste: list(int);
ma_liste.push(1);
ma_liste.push(3); // ma_liste : [1, 3]
let element_supprime = ma_liste.remove(1); // element_supprime = 3
// ma_liste devient [1]
```

**Erreurs :**
- Index négatif
- Index supérieur ou égal à la taille de la liste

## Exemple d'utilisation complète

```althread
main {
    let processus: list(proc(A));
    
    // Créer et ajouter des processus
    for i in 0..3 {
        let p = run A(i);
        processus.push(p);
    }
    
    print("Nombre de processus:", processus.len());
    
    // Accéder aux processus
    for i in 0..processus.len() {
        let p = processus.at(i);
        print("Processus à l'index", i, ":", p);
    }
    
    // Supprimer le dernier processus
    if processus.len() > 0 {
        let dernier = processus.remove(processus.len() - 1);
        print("Processus supprimé:", dernier);
    }
}
```
