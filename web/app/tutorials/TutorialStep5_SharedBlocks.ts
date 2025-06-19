import { type TutorialStep } from '../Tutorial';

export const tutorial: TutorialStep = {
  name: "sharedBlocks",
  displayName: "4. Shared Blocks",
  content: `
# 4. Shared Blocks & Shared Variables

As introduced earlier, \`shared\` blocks are crucial for concurrent programming in Althread. They define variables that can be accessed and modified by multiple processes simultaneously.

Key points:
*   Declared with the \`shared { ... }\` syntax.
*   Variables inside a \`shared\` block **must** start with an uppercase letter.
*   Shared variables **can** have a type annotation (e.g., \`MY_VAR: int\`, \`FLAG: bool\`).
*   If the have a type annotation without value, they are initialized with a default value (0, false, or empty string).

\`\`\`althread
shared {
    let GLOBAL_COUNT: int = 0;
    let APP_STATE: string = "INITIALIZING";
    let IS_ACTIVE: bool = true;
}

program Worker1 ({
    // Can read and write to GLOBAL_COUNT, APP_STATE, IS_ACTIVE
    GLOBAL_COUNT = GLOBAL_COUNT + 1;
}

program Worker2() {
    if IS_ACTIVE {
        print(APP_STATE);
    }
}

main {
    run Worker1();
    run Worker2();
}
\`\`\`
This mechanism allows different parts of your Althread application to coordinate and share data safely.

Declare a shared block containing a variable \`Counter\` of type \`int\` initialized to \`0\`, and another variable \`Message\` of type \`string\` initialized to \`"Hello from shared!"\`.
  `,
  defaultCode: `// Define your shared block here

main {
    // You can try to print them
    // print(Counter);
    // print(Message);
}`,
  validate: (code: string) => {
    const hasSharedBlockWithVars = /shared\s*{[^}]*(const|let)\s+Counter\s*(:\s*int\s*)?=\s*0;[^}]*(const|let)\s+Message\s*(:\s*string\s*)?=\s*"Hello from shared!";[^}]*}/s.test(code);
    
    if (hasSharedBlockWithVars) {
        return { success: true, message: "Shared block and variables declared correctly!" };
    }

    let issues = [];
    const hasSharedOpening = /shared\s*\{/.test(code);
    if (!hasSharedOpening) {
        issues.push("presence of 'shared {' block");
    } else {
        const hasCounter = /Counter\s*(:\s*int\s*)?(=\s*0)?;/.test(code);
        const isCounterInShared = /shared\s*{[^}]*Counter\s*(:\s*int\s*)?(=\s*0)?;[\s\S]*}/s.test(code);
        if (!hasCounter || !isCounterInShared) {
            issues.push("shared variable 'Counter: int = 0;' inside the shared block");
        }

        const hasMessage = /Message\s*(:\s*string\s*)?=\s*"Hello from shared!";/.test(code);
        const isMessageInShared = /shared\s*{[^}]*Message\s*(:\s*string\s*)?=\s*"Hello from shared!";[\s\S]*}/s.test(code);
        if (!hasMessage || !isMessageInShared) {
            issues.push("shared variable 'Message: string = \"Hello from shared!\";' inside the shared block");
        }
        
        if (issues.length === 0 && !hasSharedBlockWithVars) {
             issues.push("correct structure of the shared block with both Counter and Message variables as specified (check order or extra content).");
        }
    }
    return { success: false, message: `Check your shared block: ${issues.join(', ')}.` };
  }
};
