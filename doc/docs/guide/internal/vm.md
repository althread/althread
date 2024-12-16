---
sidebar_position: 2
---

# Machine virtuelle Althread

## Etat de la machine virtuelle

L'état de la machine virtuelle est représenté par 
- La valeur des variables globales d'un côté
- L'état des canaux de communications (les messages en transit) 
- Une structure pour chaque processus en cours d'exécution. L'état d'un processus en exécution contient sa pile d'exécution, c'est-à-dire les variables locales des processus, et la valeur du pointeur d'instruction, qui est l'index de l'instruction en cours d'exécution.

La pile d'execution d'un processus ne contient pas d'information de débuggage, elle contient uniquement les valeurs des variables locales et les valeurs intermédiaires des expressions sous forme d'un tableau de `Literal`.
Les expressions utilisant des variables locales utilisent l'index de la variable dans la pile d'exécution du processus. L'index d'une variable locale est déterminé lors de la compilation, ce qui permet un accès rapide durant l'exécution.

Pour simplifier, les variables globales sont stockés dans une HashMap (un dictionnaire) et leur valeur est directement accessible par leur nom.


## Instructions

Les instructions de la machine virtuelles sont représentées par un enum `InstructionType`. Chaque instruction contient des champs avec les informations nécessaires pour l'exécution de l'instruction:

export const InstructionType = ({children, color, code, args}) => (
    <div id={code}>
        <div
            style={{
                fontFamily: "var(--ifm-font-family-monospace)",
                fontSize: "var(--ifm-code-font-size)",
                color: "#008cdf",
            }}><span
                style={{
                    color: "#008cdf",
                }}>
                    {code}
                </span>{' '}
                <span
                style={{
                    color: "rgb(174, 0, 223)",
                }}>
                    {args}
                </span>
            </div>
        <div
        style={{
            borderRadius: '2px',
            padding: "0 15px",
        }}>
        {children}
        </div>
    </div>
);

<table>
<tr><td>
- [Empty](#Empty)
- [Expression](#Expression)
- [Push](#Push)
- [Unstack](#Unstack)
- [Destruct](#Destruct)
- [GlobalReads](#GlobalReads)
- [GlobalAssignment](#GlobalAssignment)
- [LocalAssignment](#LocalAssignment)
</td><td>
- [Declaration](#Declaration)
- [RunCall](#RunCall)
- [FnCall](#FnCall)
- [JumpIf](#JumpIf)
- [Jump](#Jump)
- [Break](#Break)
- [ChannelPeek](#ChannelPeek)
- [ChannelPop](#ChannelPop)
</td><td>
- [WaitStart](#WaitStart)
- [Wait](#Wait)
- [Send](#Send)
- [Connect](#Connect)
- [AtomicStart](#AtomicStart)
- [AtomicEnd](#AtomicEnd)
- [EndProgram](#EndProgram)
- [Exit](#Exit)
</td></tr>
</table>


<InstructionType code="Empty">
    Instruction vide, ne fait rien.
</InstructionType>

<InstructionType code="Expression" args="(LocalExpressionNode)">
    Évalue une expression et ajoute le résultat sur la pile. `LocalExpressionNode` est la racine d'un arbre représentant l'expression.
</InstructionType>

<InstructionType code="Push" args="(Literal)">
Ajoute le literal donné sur la pile.
</InstructionType>

<InstructionType code="Unstack" args="{unstack_len: usize}">
Retire `unstack_len` valeurs de la pile.
</InstructionType>

<InstructionType code="Destruct" args="">
Remplace le tuple situé au sommet de la pile par ses éléments. Un tuple contenant 3 éléments sera remplacé par 3 valeurs sur la pile.
</InstructionType>

<InstructionType code="GlobalReads" args="{variables: Vec<String>, only_const: bool}">
Ajoute les valeurs des variables globales sur la pile. Le champs `only_const` indique si les variables sont toutes des constantes (si c'est le cas, l'instruction peut être optimisée car elle n'est pas globale).
</InstructionType>

<InstructionType code="GlobalAssignment" args="{identifier: String, operator: BinaryAssignmentOperator, unstack_len: usize}">
Assigne la valeur au sommet de la pile à la variable globale `identifier` donnée, puis retire `unstack_len` valeurs de la pile.
</InstructionType>

<InstructionType code="LocalAssignment" args="{index: usize, operator: BinaryAssignmentOperator, unstack_len: usize}">
Assigne la valeur au sommet de la pile à la variable locale d'index `index` donnée, puis retire `unstack_len` valeurs de la pile.
</InstructionType>



<InstructionType code="Declaration" args="{unstack_len: usize}">
Déclare une variable dans le scope courant initialisée avec la valeur au sommet de la pile, puis retire `unstack_len` valeurs de la pile.
</InstructionType>

<InstructionType code="RunCall" args="{name: String, unstack_len: usize}">
Démarre un nouveau thread exécutant le programme `name` avec comme argument la valeur au sommet de la pile, puis retire `unstack_len` valeurs de la pile. Finalement, ajoute le pid du thread sur la pile.
</InstructionType>

<InstructionType code="FnCall" args="{name: String, unstack_len: usize, variable_idx: Option<usize>, arguments: Option<Vec<usize>}">
Appelle la fonction `name` avec les arguments locaux aux indexes donnés, puis retire `unstack_len` valeurs de la pile. Si `variable_idx` est donné, la fonction est une méthode de l'objet à l'index donné.
Finalement, ajoute le résultat de la fonction sur la pile, si elle retourne une valeur.
</InstructionType>

<InstructionType code="JumpIf" args="{jump_false: i64, unstack_len: usize}">
Saute à l'instruction `jump_false` si la valeur au sommet de la pile est fausse, puis retire `unstack_len` valeurs de la pile. La valeur du saut est relative à l'instruction courante: si `jump_false` vaut `-2`, alors le saut se fait à l'instruction courante moins 2.
</InstructionType>

<InstructionType code="Jump" args="{jump: i64}">
Saute à l'instruction `jump`. La valeur du saut est relative à l'instruction courante.
</InstructionType>

<InstructionType code="Break" args="{jump: i64, unstack_len: usize, stop_atomic: bool}">
Saute à l'instruction `jump` en retirant `unstack_len` valeurs de la pile. Si `stop_atomic` est vrai, arrête l'exécution atomique.
</InstructionType>

<InstructionType code="ChannelPeek" args="{channel_name: String}">
Regarde si un message est disponible dans le canal `channel_name`. Si c'est le cas ajoute le message et la valeur `true` sur la pile, sinon ajoute `false` sur la pile.
</InstructionType>

<InstructionType code="ChannelPop" args="{channel_name: String}">
Retire le message du canal `channel_name` (sans l'ajouter sur la pile).
</InstructionType>



<InstructionType code="WaitStart" args="{dependencies: WaitDependency, start_atomic: bool}">
Démarre une attente sur une conditions utilisant les dépendances données. Si `start_atomic` est vrai, démarre une section atomique. Les dépendances sont des variables globales ou des canaux dont la valeur est utilisée dans la condition. Cette instruction ne modifie pas l'état de la machine virtuelle, elle est utilisée uniquement pour indiqué l'entrée dans une zone d'attente. De plus un processus en attente attendra obligatoirement sur cette instruction. Cependant, ce n'est pas cette instruction seule qui permet de déterminé si le processus est en attente. L'attente est déterminée par la l'exécution de l'instruction `Wait` qui doit renvoyer le processus en attente sur l'instruction `WaitStart`.
</InstructionType>

<InstructionType code="Wait" args="{jump: i64, unstack_len: usize}">
Si le sommet de la pile est `false`, saute à l'instruction `jump` (donné en position relativement à l'instruction suivante), sinon passe à l'instruction suivante.
Dans tous les cas `unstack_len` valeurs sont dépilées.
</InstructionType>

<InstructionType code="Send" args="{channel_name: String, unstack_len: usize}">
Envoie la valeur au sommet de la pile dans le canal `channel_name`, puis retire `unstack_len` valeurs de la pile.
</InstructionType>

<InstructionType code="Connect" args="{sender_pid: Option<usize>, receiver_pid: Option<usize>, sender_channel: String, receiver_channel: String}">
Connecte les canaux `sender_channel` et `receiver_channel` entre les processus `sender_pid` et `receiver_pid`. Si `sender_pid` ou `receiver_pid` est `None`, alors le processus courant est utilisé.
Lors de la connexion, si le processus envoyeur à déjà effectué un `Send` sur le canal, le message est directement déposé dans le channel du processus receveur.
</InstructionType>

<InstructionType code="AtomicStart" args="">
Démarre une section atomique. Les sections atomiques sont des zones de code où les processus ne peuvent pas être interrompus. Cela permet d'éviter les problèmes de concurrence. Une section atomique est terminée par une instruction `AtomicEnd`. Elle ne doit pas contenir d'instructions d'attente, sauf si c'est la première instruction.
</InstructionType>

<InstructionType code="AtomicEnd" args="">
Termine une section atomique.
</InstructionType>

<InstructionType code="EndProgram" args="">
Termine le processus courant.
</InstructionType>

<InstructionType code="Exit" args="">
Termine tous les processus.
</InstructionType>


L'enum `InstructionType` est défini dans le fichier [/vm/src/instruction.rs](https://github.com/althread/althread/blob/main/interpreter/src/vm/instruction.rs#L12).
