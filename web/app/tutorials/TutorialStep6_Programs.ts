import { type TutorialStep } from '../Tutorial';

export const tutorial: TutorialStep = {
  name: "programs",
  displayName: "6. Programs (Creation & Execution)",
  content: `
# 6. Programs: Creation and Execution

In Althread, concurrent units of execution are called **programs**. You define a program using the \`program\` keyword, followed by its name, its arguments, and a block of code.

\`\`\`althread
program MyTask() {
    // Code for this program
    print("MyTask is running!");
}
\`\`\`

To execute a program, you use the \`run\` keyword. A program is executed in a **process** that runs concurrently with the main program and other processes.
\`\`\`althread
main {
    run MyTask(); // Starts a process executing MyTask program
    run MyTask(); // Starts another process executing MyTask
}
\`\`\`
Each \`run\` command creates and starts a new, independent proces of the program. These processes run concurrently.

The \`main\` block itself is the entry point of your Althread application and can also be thought of as a special process.

Define a program named \`Greeter\` that prints "Hello from Greeter program!". Then, in the \`main\` block, run this \`Greeter\` program twice.
  `,
  defaultCode: `// Define the Greeter program here

main {
    // Run the Greeter program twice
}`,
  validate: (code: string) => {
    const definesGreeter = /program\s+Greeter\s*\(\s*\)\s*{[\s\S]*print\("Hello from Greeter program!"\);[\s\S]*}/s.test(code);
    const runCount = (code.match(/run\s+Greeter\s*\(\s*\);/g) || []).length;
    const runsGreeterTwice = runCount >= 2;
    const runInMain = /main\s*{[\s\S]*(run\s+Greeter\s*\(\s*\)\s*;[\s\S]*run\s+Greeter\s*\(\s*\);|run\s+Greeter\s*\(\s*\);[^}]*run\s+Greeter\s*\(\s*\)\s*;)[\s\S]*}/s.test(code);

    if (definesGreeter && runsGreeterTwice && runInMain) {
        return { success: true, message: "Program defined and run correctly!" };
    }
    let issues = [];
    if (!definesGreeter) issues.push("definition of 'Greeter' program that prints the message 'Hello from Greeter program!'");
    if (!runsGreeterTwice) issues.push(`running 'Greeter' program at least twice (found ${runCount} runs)`);
    else if (!runInMain) issues.push("ensuring both 'run Greeter();' commands are within the main block");
    return { success: false, message: `Please check: ${issues.join(', ')}.` };
  }
};
