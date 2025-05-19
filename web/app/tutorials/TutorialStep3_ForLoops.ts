import { type TutorialStep } from '../Tutorial';

export const tutorial: TutorialStep = {
  name: "forLoops",
  displayName: "3. Loops (for)",
  content: `
# 3. Loops: \`for\`

Althread provides \`for\` loops to iterate over a range of numbers. The syntax is:

\`\`\`althread
for variable_name in start_inclusive..end_exclusive {
    // code to execute in each iteration
    // variable_name will take values from start_inclusive to end_exclusive - 1
}
\`\`\`

**Example:**
\`\`\`althread
main {
    for i in 0..5 { // i will be 0, 1, 2, 3, 4
        print(i);
    }
}
\`\`\`

Try writing a \`for\` loop that iterates from \`1\` up to (but not including) \`6\`, printing the square of each number.
For example, for \`i = 2\`, it should print \`4\`.
  `,
  defaultCode: `main {
    // Add your for loop here
    // for i in 1..6 {
    //     let square = i * i;
    //     print(square);
    // }
}`,
  validate: (code: string) => {
    const loopRegex = /for\s+(\w+)\s+in\s+1\.\.6\s*{([\s\S]*?)}/s;
    const loopMatch = code.match(loopRegex);

    if (loopMatch) {
        const loopVar = loopMatch[1];
        const loopBody = loopMatch[2];
        
        const calculatesSquareRegex = new RegExp(`let\\s+square\\s*=\\s*${loopVar}\\s*\\*\\s*${loopVar};|print\\(\\s*${loopVar}\\s*\\*\\s*${loopVar}\\s*\\)`, 's');
        const printsSquareRegex = new RegExp(`print\\(square\\);|print\\(\\s*${loopVar}\\s*\\*\\s*${loopVar}\\s*\\)`, 's');

        const calculatesSquare = calculatesSquareRegex.test(loopBody);
        const printsSquare = printsSquareRegex.test(loopBody);

        if (calculatesSquare && printsSquare) {
            return { success: true, message: "For loop implemented correctly!" };
        }
    }

    let issues = [];
    if (!loopMatch) {
        issues.push("for loop structure 'for i in 1..6 {}'");
    } else {
        const loopVar = loopMatch[1];
        const loopBody = loopMatch[2];
        const calculatesSquareRegex = new RegExp(`let\\s+square\\s*=\\s*${loopVar}\\s*\\*\\s*${loopVar};|print\\(\\s*${loopVar}\\s*\\*\\s*${loopVar}\\s*\\)`, 's');
        const printsSquareRegex = new RegExp(`print\\(square\\);|print\\(\\s*${loopVar}\\s*\\*\\s*${loopVar}\\s*\\)`, 's');

        if (!calculatesSquareRegex.test(loopBody)) {
            issues.push(`calculation of the square (e.g., let square = ${loopVar} * ${loopVar}; or print(${loopVar}*${loopVar});) inside the loop`);
        }
        if (!printsSquareRegex.test(loopBody)) {
            issues.push("printing the square (e.g., print(square) or print(i*i)) inside the loop");
        }
    }
    return { success: false, message: `Review your loop: ${issues.join(', ')}.` };
  }
};
