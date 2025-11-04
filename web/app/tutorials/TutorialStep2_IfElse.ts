import { type TutorialStep } from '@components/tutorial/Tutorial';

export const tutorial: TutorialStep = {
  name: "ifElse",
  displayName: "2. Conditional Logic (if/else)",
  content: `
# 2. Conditional Logic: \`if\`, \`else if\`, \`else\`

Althread uses \`if\`, \`else if\`, and \`else\` statements for conditional execution, similar to many other programming languages.

The basic structure is:
\`\`\`althread
if condition1 {
    // code to execute if condition1 is true
} else if condition2 {
    // code to execute if condition1 is false and condition2 is true
} else {
    // code to execute if all preceding conditions are false
}
\`\`\`
Conditions are boolean expressions.

**Example:**
\`\`\`althread
main {
    let temperature = 25;
    if temperature > 30 {
        print("It's hot!");
    } else if temperature > 20 {
        print("It's warm.");
    } else {
        print("It's cool or cold.");
    }
}
\`\`\`

Try writing an \`if/else\` statement that checks a local variable \`x\`.
If \`x\` is greater than 10, print "x is greater than 10".
Otherwise, print "x is not greater than 10".
Initialize \`x\` to 7.
  `,
  defaultCode: `main {
    let x = 7;

    // Add your if/else statement here
}`,
  validate: (code: string) => {
    const hasLetX = /let\s+x\s*=\s*7;/.test(code);
    const hasIfCondition = /if\s+x\s*>\s*10\s*{/.test(code);
    const hasPrintGreaterInIf = /if\s+x\s*>\s*10\s*{[^}]*print\("x is greater than 10"\);[^}]*}/s.test(code);
    const hasElseWithPrintNotGreater = /}\s*else\s*{[^}]*print\("x is not greater than 10"\);[^}]*}/s.test(code);

    if (hasLetX && hasPrintGreaterInIf && hasElseWithPrintNotGreater) {
        return { success: true, message: "Great job with the if/else statement!" };
    }
    let issues = [];
    if (!hasLetX) issues.push("initialize 'x' to 7");
    if (!hasIfCondition) issues.push("if condition 'x > 10'");
    if (!hasPrintGreaterInIf) issues.push("print statement 'x is greater than 10' inside the if block");
    if (!hasElseWithPrintNotGreater) issues.push("else block with print statement 'x is not greater than 10'");
    return { success: false, message: `Check the following: ${issues.join(', ')}.` };
  }
};
