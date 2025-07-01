import { type TutorialStep } from '@components/tutorial/Tutorial';

export const tutorial: TutorialStep = {
  name: "functions",
  displayName: "4. Functions",
  content: `
# 4. Functions

Functions are reusable blocks of code that perform a specific task. In Althread, you define them using the \`fn\` keyword. They can also call themselves, which is known as recursion.

## Syntax
A function definition includes its name, a list of parameters with their types, a return type, and a body.

\`\`\`althread
fn function_name(param1: type, param2: type) -> return_type {
    // code block
    return some_value; // must match return_type
}
\`\`\`

*   \`fn\`: The keyword to start a function definition.
*   \`function_name\`: The name of your function.
*   \`parameters\`: A comma-separated list of \`name: type\`.
*   \`-> return_type\`: Specifies the type of the value the function will return.
*   \`return\`: The keyword to send a value back from the function.

**Note on Parameters:** When you pass a variable to a function, a *copy* of its value is used (this is known as "pass-by-value"). Any changes to the parameter inside the function will not affect the original variable outside the function.

## Functions Without a Return Value
If a function does not need to return a value, you can omit the return type specification (\`-> return_type\`).

\`\`\`althread
fn log_message(message: string) {
    print("Log: " + message);
}
\`\`\`

You can use an empty \`return;\` statement to exit such a function early. If there is no \`return\` statement, the function automatically returns after its last statement is executed.

**Example:**
\`\`\`althread
fn add(a: int, b: int) -> int {
    return a + b;
}

main {
    let sum = add(5, 3);
    print("The sum is: " + sum); // Prints "The sum is: 8"
}
\`\`\`

## Your Task:
1.  Define a function named \`multiply\` that takes two integer parameters.
2.  The function should return the product of these two numbers.
3.  In the \`main\` block, call your \`multiply\` function with two numbers (e.g., 7 and 6) and print the result.
  `,
  defaultCode: `// Define the 'multiply' function here

main {
    // Call the function and print the result
}`,
  validate: (code: string) => {
    const funcRegex = /fn\s+multiply\s*\(\s*(\w+)\s*:\s*int\s*,\s*(\w+)\s*:\s*int\s*\)\s*->\s*int\s*\{([\s\S]*?)\}/s;
    const funcMatch = code.match(funcRegex);

    let isFuncCorrect = false;
    if (funcMatch) {
        const param1 = funcMatch[1];
        const param2 = funcMatch[2];
        const body = funcMatch[3];
        const returnRegex = new RegExp(`return\\s+${param1}\\s*\\*\\s*${param2}\\s*;|return\\s+${param2}\\s*\\*\\s*${param1}\\s*;`, 's');
        if (returnRegex.test(body)) {
            isFuncCorrect = true;
        }
    }

    const mainContentMatch = code.match(/main\s*\{([\s\S]*?)\}/s);
    let isMainCorrect = false;
    if (mainContentMatch) {
        const mainContent = mainContentMatch[1];
        // Case 1: print(multiply(x, y))
        const directPrint = /print\([\s\S]*multiply\(\s*\d+\s*,\s*\d+\s*\)[\s\S]*\)/s.test(mainContent);
        // Case 2: let result = multiply(x, y); print(result)
        const varPrintMatch = mainContent.match(/let\s+(\w+)\s*=\s*multiply\(\s*\d+\s*,\s*\d+\s*\)\s*;/s);
        let indirectPrint = false;
        if (varPrintMatch) {
            const varName = varPrintMatch[1];
            const printVarRegex = new RegExp(`print\\([\\s\\S]*${varName}[\\s\\S]*\\)`, 's');
            indirectPrint = printVarRegex.test(mainContent);
        }
        if (directPrint || indirectPrint) {
            isMainCorrect = true;
        }
    }


    if (isFuncCorrect && isMainCorrect) {
        return { success: true, message: "Function defined and called correctly!" };
    }

    let issues = [];
    if (!funcMatch) {
        issues.push("defining a function 'multiply(a: int, b: int) -> int'");
    } else if (!isFuncCorrect) {
        issues.push("the body of 'multiply' to return the product of its parameters (e.g., 'return a * b;')");
    }
    if (!isMainCorrect) {
        issues.push("calling 'multiply' from the main block and printing its result");
    }
    
    return { success: false, message: `Please check: ${issues.join(', ')}.` };
  }
};