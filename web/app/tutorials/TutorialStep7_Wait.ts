import { type TutorialStep } from '@components/tutorial/Tutorial';

export const tutorial: TutorialStep = {
  name: "wait",
  displayName: "7. Awaiting Conditions (wait until)",
  content: `
# 7. Awaiting Conditions: \`await condition\`

The \`await\` statement is a powerful construct in Althread for synchronization. It allows a program to pause its execution until a specific condition becomes true. This is often used with shared variables.

Syntax:
\`\`\`althread
await <boolean_condition>;
\`\`\`
The program will block at this line and only resume when the \`<boolean_condition>\` evaluates to \`true\`.

**Example:**
\`\`\`althread
shared {
    let Flag: bool = false;
}

program Setter() {
    Flag = true;
    print("Setter: Flag is now true.");
}

program Waiter() {
    print("Waiter: Waiting for Flag to be true...");
    await Flag == true; // or simply 'await Flag;' if Flag is boolean
    print("Waiter: Flag is true! Resuming.");
}

main {
    run Setter();
    run Waiter();
}
\`\`\`
In this example, \`Waiter\` will pause until \`Setter\` changes \`Flag\` to \`true\`.

**Your Task:**
1. Create a shared boolean variable \`READY\` initialized to \`false\`.
2. Create a program \`Signal\` that sets \`READY\` to \`true\` and prints "Signal: READY set to true".
3. Create a program \`Listener\` that prints "Listener: Waiting..." and wait \`READY\` to become true, then prints "Listener: Ready!".
4. Run both programs in \`main\`.
  `,
  defaultCode: `
main {

}`,
  validate: (code: string) => {
    const hasSharedReady = /shared\s*{[^}]*READY\s*:\s*bool\s*=\s*false;[^}]*}/s.test(code);
    const hasProgramSignal = /program\s+Signal\s*\(\s*\)\s*{[^}]*READY\s*=\s*true;[^}]*print\("Signal: READY set to true"\);[^}]*}/s.test(code);
    const hasProgramListener = /program\s+Listener\s*\(\s*\)\s*{[^}]*print\("Listener: Waiting..."\);[^}]*await\s+READY(\s*==\s*true)?;[^}]*print\("Listener: Ready!"\);[^}]*}/s.test(code);
    const runsBothInMain = /main\s*{[\s\S]*(run\s+Signal\s*\(\s*\)\s*;[\s\S]*run\s+Listener\s*\(\s*\)\s*;|run\s+Listener\s*\(\s*\)\s*;[\s\S]*run\s+Signal\s*\(\s*\)\s*;)[\s\S]*}/s.test(code);


    if (hasSharedReady && hasProgramSignal && hasProgramListener && runsBothInMain) {
        return { success: true, message: "\`await\` implemented correctly!" };
    }
    let issues = [];
    if (!hasSharedReady) issues.push("shared variable 'READY: bool = false'");
    if (!hasProgramSignal) issues.push("Program 'Signal' to set READY to true, and print 'Signal: READY set to true'");
    if (!hasProgramListener) issues.push("Program 'Listener' to print 'Listener: Waiting...', wait for READY (e.g. 'await READY;'), and print 'Listener: Ready!'");
    if (!runsBothInMain) issues.push("running both Signal and Listener programs in the main block");
    return { success: false, message: `Check the following: ${issues.join(', ')}.` };
  }
};
