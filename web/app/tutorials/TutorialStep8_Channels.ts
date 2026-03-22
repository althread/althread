import type { TutorialStep } from "@components/tutorial/Tutorial";

export const tutorial: TutorialStep = {
	name: "channels",
	displayName: "8. Channels (Declaration, Send, Receive)",
	content: `
# 8. Channels: Communication Between Programs

Channels provide a way for Althread programs to communicate by sending and receiving messages. They are typed, meaning a channel is declared to carry messages of a specific type.

## Declaration
Channels are usually created in the main block, as we need to know what processes they connect. They connect a process **out port** to a process **in port**. The name a port can be an arbitrary identifier. The syntax for declaring a channel is:
\`\`\`althread
main {
    process1 = run Prog1();
    process2 = run Prog2();
    channel process1.outPortName (type1, type2, ...)> process2.inPortName;
}
\`\`\`

## Sending Messages
Use the \`send\` keyword to send a value to a channel:
\`\`\`althread
send outPortName(value1, value2, ...);
\`\`\`
Sending is not a blocking operation as we assume channel have infinite capacity.

## Receiving Messages
Use the \`receive\` keyword to receive a value from a channel. This is not alone a blocking operation, but it can be seen as a boolean condition that is true when a message is correctly received. So it is tipically used in an \`await\` statement:
\`\`\`althread
await receive inPortName(value1, value2, ...);
// Do something with the received values
\`\`\`

**Example: Producer-Consumer**
\`\`\`althread

program Producer() {
    for i in 0..3 {
        print("Producer: sending ", i);
        send outPort(i);
    }
}

program Consumer() {
    for i in 0..3 { // We expect 3 messages
        await receive inPort(val);
        print("Consumer: received ", val);
    }
}

main {
    let prod = run Producer();
    let cons = run Consumer();
    channel prod.outPort (int)> cons.inPort;
}
\`\`\`

## Your Task:
Create a \`Sender\` program that sends the message \`"Ping"\` and prints "Sender: sent 'Ping'".
Create a \`Receiver\` program that receives a message from an in port into a variable \`msg\` and prints "Receiver: received " followed by that variable.
Run both programs and create a channel between them with the correct in and out port name.
  `,
	defaultCode: `
main {

}`,
	validate: (code: string) => {
		const issues = [];

		// Helper function to escape strings for RegExp constructor
		const escapeRegExp = (str: string) =>
			str.replace(/[.*+?^${}()|[\]\\]/g, "$&");

		// 1. Validate Sender program and capture its out-port name
		const senderProgramRegex =
			/program\s+Sender\s*\(\s*\)\s*\{[^}]*send\s+(\w+)\s*\(\s*"Ping"\s*\)\s*;[^}]*print\s*\(\s*"Sender: sent 'Ping'"\s*\)\s*;[^}]*\}/s;
		const senderMatch = code.match(senderProgramRegex);
		const isSenderProgramCorrect = senderMatch !== null;
		const senderOutPort = isSenderProgramCorrect ? senderMatch[1] : null;

		if (!isSenderProgramCorrect) {
			issues.push(
				"Sender Program: Must be 'program Sender()', send '\"Ping\"' via an out port (e.g., 'send myOutPort(\"Ping\");'), and print \"Sender: sent 'Ping'\".",
			);
		}

		// 2. Validate Receiver program and capture its in-port name and message variable
		const receiverProgramRegex =
			/program\s+Receiver\s*\(\s*\)\s*\{[^}]*await\s+receive\s+(\w+)\s*\(\s*(\w+)\s*\)\s*;[^}]*print\s*\(\s*"Receiver: received "\s*,\s*\2\s*\)\s*;[^}]*\}/s;
		const receiverMatch = code.match(receiverProgramRegex);
		const isReceiverProgramCorrect = receiverMatch !== null;
		const receiverInPort = isReceiverProgramCorrect ? receiverMatch[1] : null;
		const receiverMessageVariable = isReceiverProgramCorrect
			? receiverMatch[2]
			: null;

		if (!isReceiverProgramCorrect) {
			issues.push(
				"Receiver Program: Must be 'program Receiver()', use 'await receive myInPort(msg);' on an in port, and print '\"Receiver: received \"' followed by the received variable (e.g., 'print(\"Receiver: received \", msg);').",
			);
		} else if (
			receiverMessageVariable &&
			!/^[a-zA-Z_]\w*$/.test(receiverMessageVariable)
		) {
			issues.push(
				`Receiver Program: The variable used in 'receive ${receiverInPort}(${receiverMessageVariable})' (\`${receiverMessageVariable}\`) is not a valid identifier.`,
			);
		}

		// 3. Validate main block
		let isMainBlockCorrect = false;
		const mainBlockContentMatch = code.match(/main\s*\{([\s\S]*?)\}/s);

		if (!mainBlockContentMatch) {
			issues.push("Main Block: A 'main { ... }' block is required.");
		} else {
			const mainContent = mainBlockContentMatch[1];

			const senderRunRegex = /let\s+(\w+)\s*=\s*run\s+Sender\s*\(\s*\)\s*;/s;
			const receiverRunRegex =
				/let\s+(\w+)\s*=\s*run\s+Receiver\s*\(\s*\)\s*;/s;

			const senderRunMatch = mainContent.match(senderRunRegex);
			const receiverRunMatch = mainContent.match(receiverRunRegex);

			const senderInstanceName = senderRunMatch ? senderRunMatch[1] : null;
			const receiverInstanceName = receiverRunMatch
				? receiverRunMatch[1]
				: null;

			const mainBlockSpecificIssues = [];

			if (!senderInstanceName) {
				mainBlockSpecificIssues.push(
					"Sender execution: 'let senderInstance = run Sender();' in main.",
				);
			}
			if (!receiverInstanceName) {
				mainBlockSpecificIssues.push(
					"Receiver execution: 'let receiverInstance = run Receiver();' in main.",
				);
			}

			if (
				isSenderProgramCorrect &&
				isReceiverProgramCorrect &&
				senderInstanceName &&
				receiverInstanceName &&
				senderOutPort &&
				receiverInPort
			) {
				const expectedChannelString = `channel ${senderInstanceName}.${senderOutPort} (string) > ${receiverInstanceName}.${receiverInPort};`;
				// Need to escape for regex. Example: instance.port(string)>instance2.port2;
				const channelRegexString = `channel\\s+${escapeRegExp(senderInstanceName)}\\.${escapeRegExp(senderOutPort)}\\s*\\(\\s*string\\s*\\)\\s*>\\s+${escapeRegExp(receiverInstanceName)}\\.${escapeRegExp(receiverInPort)}\\s*;`;
				const channelRegex = new RegExp(channelRegexString, "s");

				if (!channelRegex.test(mainContent)) {
					mainBlockSpecificIssues.push(
						`Channel: Declare '${expectedChannelString}' in main.`,
					);
				} else {
					isMainBlockCorrect = true;
				}
			} else {
				// This block provides feedback if parts are missing for full channel validation
				if (!mainContent.includes("channel ")) {
					mainBlockSpecificIssues.push(
						"Channel: A channel declaration is missing in main.",
					);
				} else if (senderOutPort && receiverInPort) {
					// Attempt to find a channel with the correct port names but possibly wrong instance names or type
					const genericChannelRegexString = `channel\\s+\\w+\\.${escapeRegExp(senderOutPort)}\\s*\\(\\s*string\\s*\\)\\s*>\\s+\\w+\\.${escapeRegExp(receiverInPort)}\\s*;`;
					const genericChannelRegex = new RegExp(
						genericChannelRegexString,
						"s",
					);
					if (!genericChannelRegex.test(mainContent)) {
						mainBlockSpecificIssues.push(
							`Channel: A channel of type 'string' connecting an out port named '${senderOutPort}' to an in port named '${receiverInPort}' seems to be missing or incorrect.`,
						);
					} else {
						mainBlockSpecificIssues.push(
							`Channel: A channel using ports '${senderOutPort}' and '${receiverInPort}' exists, but make sure it connects the correct instances of Sender and Receiver (defined with 'let ... = run ...;'), and that the type is 'string'.`,
						);
					}
				} else if (issues.length === 0) {
					// Only add this generic message if no other specific program errors were found
					mainBlockSpecificIssues.push(
						"Channel: The declaration in main could not be fully validated. Ensure Sender/Receiver programs are correct, executed with 'let instance = run Program();', and then connected by a channel.",
					);
				}
			}

			if (mainBlockSpecificIssues.length > 0) {
				issues.push(`Main block issues: ${mainBlockSpecificIssues.join(" ")}`);
			}
		}

		if (
			isSenderProgramCorrect &&
			isReceiverProgramCorrect &&
			isMainBlockCorrect
		) {
			return {
				success: true,
				message: "Channels used correctly for sending/receiving!",
			};
		} else {
			const finalIssues = [...new Set(issues)]; // Remove duplicates
			if (
				finalIssues.length === 0 &&
				!(
					isSenderProgramCorrect &&
					isReceiverProgramCorrect &&
					isMainBlockCorrect
				)
			) {
				// This case should ideally not be reached if logic is comprehensive
				finalIssues.push(
					"An unknown validation error occurred. Please check your code against the tutorial task description.",
				);
			}
			return {
				success: false,
				message: `Please review your implementation: ${finalIssues.join(" ")}`,
			};
		}
	},
};
