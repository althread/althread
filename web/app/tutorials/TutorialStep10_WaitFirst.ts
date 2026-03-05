import { type TutorialStep } from '@components/tutorial/Tutorial';

export const tutorial: TutorialStep = {
  name: "wait-first",
  displayName: "10. Multi-Condition Wait (await first)",
  content: `
# 10. Multi-Condition Wait: await first

In concurrent programming, you often need to wait for one of several possible events. Althread provides the \`await first\` construct to handle this.

## Selective Wait
The \`await first\` statement allows a program to wait for multiple conditions simultaneously. It will block until **at least one** of the conditions becomes true. If multiple conditions are true, it executes the block associated with the **first** one that matches (from top to bottom).

The syntax is:
\`\`\`althread
await first {
    (condition1) => {
        // block executed if condition1 is true
    }
    (condition2) => {
        // block executed if condition2 is true
    }
}
\`\`\`

## Receiving from Multiple Channels
A common use case is waiting for messages from different ports:

\`\`\`althread
await first {
    receive portA(val) => {
        print("Received from A: ", val);
    }
    receive portB(val) => {
        print("Received from B: ", val);
    }
}
\`\`\`

**Note:** When used inside \`first\`, the \`receive\` condition is followed by \`=> { ... }\` to define the action to take when that specific message is received.

## Your Task:
Create a program that waits for either a message on \`port1\` or a message on \`port2\`.
- If a message is received on \`port1\`, print "Received from port 1".
- If a message is received on \`port2\`, print "Received from port 2".

In the \`main\` block, run one instance of this program and two sender programs (one for each port) to test it.
  `,
  defaultCode: `
program MultiReceiver() {
    // Write your await first here
}

program Sender1() {
    send out(1);
}

program Sender2() {
    send out(2);
}

main {
    let receiver = run MultiReceiver();
    let s1 = run Sender1();
    let s2 = run Sender2();
    channel s1.out (int)> receiver.port1;
    channel s2.out (int)> receiver.port2;
}`,
  validate: (code: string) => {
    const issues = [];

    const hasAwaitFirst = /await\s+first\s*\{/.test(code);
    if (!hasAwaitFirst) {
        issues.push("You must use 'await first { ... }' to wait for multiple conditions.");
    }

    const hasReceivePort1 = /receive\s+port1\s*\(\s*\w+\s*\)\s*=>\s*\{\s*print\s*\(\s*"Received from port 1"\s*\)\s*;?\s*\}/.test(code);
    const hasReceivePort2 = /receive\s+port2\s*\(\s*\w+\s*\)\s*=>\s*\{\s*print\s*\(\s*"Received from port 2"\s*\)\s*;?\s*\}/.test(code);

    if (!hasReceivePort1) {
        issues.push("You need to handle 'receive port1' and print 'Received from port 1'.");
    }
    if (!hasReceivePort2) {
        issues.push("You need to handle 'receive port2' and print 'Received from port 2'.");
    }

    if (issues.length === 0) {
        return { success: true, message: "Great! You've mastered multi-condition waiting." };
    } else {
        return { success: false, message: issues.join(" ") };
    }
  },
}
