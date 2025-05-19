import { type TutorialStep } from '../Tutorial';

export const tutorial: TutorialStep = {
  name: "variables",
  displayName: "1. Variables",
  content: `
# 1. Variables in Althread

Althread supports two main types of variables: local and shared.

## Local Variables

Local variables are declared using the \`let\` or \`const\` keyword and **must start with a lowercase letter**. They are scoped to the block in which they are defined (e.g., within a function, loop, or program).

\`\`\`althread
main {
    let myLocalVariable = 10;
    const anotherOne = "hello";
    print(myLocalVariable);
}
\`\`\`

## Shared Variables

Shared variables are used for communication and synchronization between different programs. They must be declared within a \`shared\` block and **their names must start with an uppercase letter**. They are also declared using the \`let\` or \`const\` keyword.

\`\`\`althread
shared {
    let SHARED_COUNTER = 0;
    let IsDone = false;
}

main {
    // SHARED_COUNTER can be accessed here
    print(SHARED_COUNTER, IsDone);
}
\`\`\`

Try declaring one local variable \`count\` initialized to \`5\` and one shared variable \`STATUS_MESSAGE\` of type \`string\` initialized to \`"Pending"\`.
  `,
  defaultCode: `
main {
    // Declare a local variable 'count'
    // You can try to print them if you like
    // print(count);
    // print(STATUS_MESSAGE);
}`,
  validate: (code: string) => {
    const hasLocalVar = /(let|const)\s+count\s*=\s*5;/.test(code);
    const hasSharedVarInBlock = /shared\s*{[^}]*(let|const)\s+STATUS_MESSAGE\s*(:\s*string\s*)?=\s*"Pending";[^}]*}/s.test(code);

    if (hasLocalVar && hasSharedVarInBlock) {
        return { success: true, message: "Excellent! Both local and shared variables declared correctly." };
    }
    let missing = [];
    if (!hasLocalVar) missing.push("local variable 'count = 5'");
    if (!hasSharedVarInBlock) missing.push("shared variable 'STATUS_MESSAGE: string = \"Pending\"' inside a shared block");
    return { success: false, message: `Looks like you're missing: ${missing.join(', ')}.` };
  }
};
