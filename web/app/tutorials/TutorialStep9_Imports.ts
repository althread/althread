import { type TutorialStep } from '@components/tutorial/Tutorial';

export const tutorial: TutorialStep = {
  name: "imports",
  displayName: "9. Imports",
  content: `
# 9. Imports

As your Althread projects grow, you'll want to organize code into multiple files. The \`import\` statement lets you bring definitions from other files, even those in subdirectories, into your current file.

## Syntax

\`\`\`althread
import [
    math,
    cool/fib,
    display
]
\`\`\`

- Each item in the list is a relative filepath (e.g., \`directory/file\`), without the \`.alt\` extension.
- When importing from a subdirectory like \`cool/fib\`, the module is available under its filename, \`fib\`.
- You can use \`as\` to give an imported module an alias.

## Available Modules

For this tutorial, we've provided three modules for you to work with:

### math.alt
\`\`\`althread
fn max(a: int, b: int) -> int {
  if a > b {
    return a;
  }
  return b;
}
\`\`\`

### cool/fib.alt
\`\`\`althread
shared {
  let N: int = 8;
}

@private
fn fibonacci_iterative(n: int, a: int, b: int) -> int {
  for i in 1..n {
    let c = a + b;
    a = b;
    b = c;
  }
  return b;
}

fn fibonacci_iterative_N() -> int {
  return fibonacci_iterative(N, 0, 1);      
}
\`\`\`

### display.alt
\`\`\`althread
program Hello() {
  print("Hi there!");
}
\`\`\`

## Using Imported Items

Once imported, you can access items from that module using dot notation. Notice how the \`@private\` directive in \`cool/fib.alt\` hides the \`fibonacci_iterative\` function from external access.

\`\`\`althread
main {
    // Call a function from 'math'
    let result = math.max(5, 10);
    print(result); // Prints 10

    // Access shared variable from 'cool/fib'
    print(fib.N); // Prints 8

    // Run a program from 'display'
    run display.Hello(); // Prints "Hi there!"
}
\`\`\`

## Your Task

1.  Import the \`math\`, \`cool/fib\`, and \`display\` modules.
2.  In the \`main\` block, call \`math.max(7, 3)\` and print the result.
3.  The module \`cool/fib\` contains a shared variable \`N\` with the value \`8\`. Call \`fib.fibonacci_iterative_N()\` and print its result. Also print the value of \`N\` directly.
4.  Change the value of \`N\` to \`10\` using \`fib.N = 10\` and call \`fib.fibonacci_iterative_N()\` again to see the updated result.
5.  Try calling \`fib.fibonacci_iterative(8, 0, 1)\` directly. You'll see an error because this function is marked as \`@private\` and cannot be accessed from outside the module.
6.  Finally, run the \`Hello()\` program from the \`display\` module.

  `,
  defaultCode: `import [
    // Import modules here
]

main {
    // Call imported functions and run programs
}`,
  validate: (code: string) => {
    // 1. Validate the import statement
    const importMatch = code.match(/import\s*\[([\s\S]*?)\]/s);
    let isImportCorrect = false;
    if (importMatch) {
        const importList = importMatch[1];
        const hasMath = /\bmath\b/.test(importList);
        const hasCoolFib = /cool\/fib/.test(importList);
        const hasDisplay = /\bdisplay\b/.test(importList);
        if (hasMath && hasCoolFib && hasDisplay) {
            isImportCorrect = true;
        }
    }

    // 2. Validate the main block content
    const mainMatch = code.match(/main\s*\{([\s\S]*?)\}/s);
    let missingSteps: string[] = [];
    let forbiddenUsage = false;

    if (mainMatch) {
        const mainContent = mainMatch[1];

        // Step 2: math.max(7, 3) and print
        const mathVar = mainContent.match(/let\s+(\w+)\s*=\s*math\.max\(\s*7\s*,\s*3\s*\)\s*;/);
        const mathDirectPrint = /print\([\s\S]*math\.max\(\s*7\s*,\s*3\s*\)[\s\S]*\)/.test(mainContent);
        const mathVarPrint = mathVar && new RegExp(`print\\([\\s\\S]*${mathVar[1]}[\\s\\S]*\\)`).test(mainContent);
        if (!(mathDirectPrint || mathVarPrint)) missingSteps.push("printing the result of math.max(7, 3)");

        // Step 3: fib.fibonacci_iterative_8() and print
        const fibVar = mainContent.match(/let\s+(\w+)\s*=\s*fib\.fibonacci_iterative_N\(\s*\)\s*;/);
        const fibDirectPrint = /print\([\s\S]*fib\.fibonacci_iterative_N\(\s*\)[\s\S]*\)/.test(mainContent);
        const fibVarPrint = fibVar && new RegExp(`print\\([\\s\\S]*${fibVar[1]}[\\s\\S]*\\)`).test(mainContent);
        if (!(fibDirectPrint || fibVarPrint)) missingSteps.push("printing the result of fib.fibonacci_iterative_8()");

        // Step 3b: print fib.N
        if (!/print\([\s\S]*fib\.N[\s\S]*\)/.test(mainContent)) missingSteps.push("printing fib.N");

        // Step 4: assign fib.N = 10 and print new fib.fibonacci_iterative_8()
        const nAssign = /fib\.N\s*=\s*10\s*;/.test(mainContent);
        if (!nAssign) missingSteps.push("assigning fib.N = 10");
        else {
            // Only check for second fib print after assignment
            const afterAssign = mainContent.split(/fib\.N\s*=\s*10\s*;/)[1] || "";
            const fibVarAfter = afterAssign.match(/let\s+(\w+)\s*=\s*fib\.fibonacci_iterative_N\(\s*\)\s*;/);
            const fibDirectPrintAfter = /print\([\s\S]*fib\.fibonacci_iterative_N\(\s*\)[\s\S]*\)/.test(afterAssign);
            const fibVarPrintAfter = fibVarAfter && new RegExp(`print\\([\\s\\S]*${fibVarAfter[1]}[\\s\\S]*\\)`).test(afterAssign);
            if (!(fibDirectPrintAfter || fibVarPrintAfter)) missingSteps.push("printing the result of fib.fibonacci_iterative_N() after changing fib.N");
        }

        // Step 6: run display.Hello()
        if (!/run\s+display\.Hello\(\s*\)\s*;/.test(mainContent)) missingSteps.push("running display.Hello()");

        // Check for forbidden usage of fib.fibonacci_iterative(8, 0, 1)
        if (/fib\.fibonacci_iterative\s*\(\s*8\s*,\s*0\s*,\s*1\s*\)/.test(mainContent)) {
            forbiddenUsage = true;
        }
    } else {
        missingSteps.push("main block");
    }

    if (!isImportCorrect) missingSteps.unshift("importing 'math', 'cool/fib', and 'display'");

    if (forbiddenUsage) {
        return {
            success: false,
            message: "You tried to call fib.fibonacci_iterative(8, 0, 1) directly. This function is private and cannot be used outside the module. Please use fib.fibonacci_iterative_N() instead."
        };
    }

    if (missingSteps.length === 0) {
        return { success: true, message: "Fantastic! You've completed all the tasks for this tutorial." };
    }

    return {
        success: false,
        message: `Please check the following: ${missingSteps.join('; ')}.`
    };
  }
};