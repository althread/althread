import { type TutorialStep } from '../Tutorial';

export const tutorial: TutorialStep = {
  name: "wait",
  displayName: "6. Awaiting Conditions (wait until)",
  content: `
# 6. Awaiting Conditions: \`wait until\`

The \`wait until\` statement is a powerful construct in Althread for synchronization. It allows a program to pause its execution until a specific condition becomes true. This is often used with shared variables.

Syntax:
\`\`\`althread
wait until <boolean_condition>;
\`\`\`
The program will block at this line and only resume when the \`<boolean_condition>\` evaluates to \`true\`.

**Example:**
\`\`\`althread
shared {
    FLAG: bool = false;
}

program Setter {
    sleep(100); // Simulate some work
    FLAG = true;
    print("Setter: FLAG is now true.");
}

program Waiter {
    print("Waiter: Waiting for FLAG to be true...");
    wait until FLAG == true; // or simply 'wait until FLAG;' if FLAG is boolean
    print("Waiter: FLAG is true! Resuming.");
}

main {
    run Setter;
    run Waiter;
}
\`\`\`
In this example, \`Waiter\` will pause until \`Setter\` changes \`FLAG\` to \`true\`.

**Your Task:**
1. Create a shared boolean variable \`READY\` initialized to \`false\`.
2. Create a program \`Signal\` that sets \`READY\` to \`true\` after a small delay (e.g. \`sleep(50);\`) and prints "Signal: READY set to true".
3. Create a program \`Listener\` that prints "Listener: Waiting..." and then uses \`wait until READY\` to become true, then prints "Listener: Ready!".
4. Run both programs in \`main\`.
  `,
  defaultCode: `shared {
    // READY: bool = false;
}

program Signal {
    // sleep(50);
    // READY = true;
    // print("Signal: READY set to true");
}

program Listener {
    // print("Listener: Waiting...");
    // wait until READY;
    // print("Listener: Ready!");
}

main {
    // run Signal;
    // run Listener;
}`,
  validate: (code: string) => {
    const hasSharedReady = /shared\s*{[^}]*READY\s*:\s*bool\s*=\s*false;[^}]*}/s.test(code);
    const hasProgramSignal = /program\s+Signal\s*{[^}]*sleep\(\s*(50|\d+)\s*\);[^}]*READY\s*=\s*true;[^}]*print\("Signal: READY set to true"\);[^}]*}/s.test(code);
    const hasProgramListener = /program\s+Listener\s*{[^}]*print\("Listener: Waiting..."\);[^}]*wait\s+until\s+READY(\s*==\s*true)?;[^}]*print\("Listener: Ready!"\);[^}]*}/s.test(code);
    const runsBothInMain = /main\s*{[\s\S]*(run\s+Signal\s*\(\s*\);[\s\S]*run\s+Listener\s*\(\s*\);|run\s+Listener\s*\(\s*\);[\s\S]*run\s+Signal\s*\(\s*\);)[\s\S]*}/s.test(code);


    if (hasSharedReady && hasProgramSignal && hasProgramListener && runsBothInMain) {
        return { success: true, message: "\`wait until\` implemented correctly!" };
    }
    let issues = [];
    if (!hasSharedReady) issues.push("shared variable 'READY: bool = false'");
    if (!hasProgramSignal) issues.push("Program 'Signal' to sleep, set READY to true, and print 'Signal: READY set to true'");
    if (!hasProgramListener) issues.push("Program 'Listener' to print 'Listener: Waiting...', wait for READY (e.g. 'wait until READY;'), and print 'Listener: Ready!'");
    if (!runsBothInMain) issues.push("running both Signal and Listener programs in the main block");
    return { success: false, message: `Check the following: ${issues.join(', ')}.` };
  }
};
