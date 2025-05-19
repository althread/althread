import { type TutorialStep } from '../Tutorial';

export const tutorial: TutorialStep = {
  name: "channels",
  displayName: "7. Channels (Declaration, Send, Receive)",
  content: `
# 7. Channels: Communication Between Programs

Channels provide a way for Althread programs to communicate by sending and receiving messages. They are typed, meaning a channel is declared to carry messages of a specific type.

## Declaration
Channels are declared globally or within a shared block, specifying their name and the type of data they will transmit:
\`\`\`althread
channel myIntChannel: int;
channel myStringChannel: string;
\`\`\`

## Sending Messages
Use the \`send\` keyword to send a value to a channel:
\`\`\`althread
send 42 to myIntChannel;
send "hello" to myStringChannel;
\`\`\`
Sending is a blocking operation if the channel has no buffer or the buffer is full.

## Receiving Messages
Use the \`recv\` keyword to receive a value from a channel. This is also typically a blocking operation, waiting until a message is available.
\`\`\`althread
let receivedNumber = recv myIntChannel;
let receivedString = recv myStringChannel;
print(receivedNumber);
\`\`\`

**Example: Producer-Consumer**
\`\`\`althread
channel dataChannel: int;

program Producer {
    for i in 0..3 {
        print("Producer: sending ", i);
        send i to dataChannel;
        sleep(10); // Small delay
    }
}

program Consumer {
    for _ in 0..3 { // We expect 3 messages
        let val = recv dataChannel;
        print("Consumer: received ", val);
    }
}

main {
    run Producer;
    run Consumer;
}
\`\`\`

Declare a channel named \`messageChan\` for \`string\` type.
Create a \`Sender\` program that sends the message \`"Ping"\` to \`messageChan\` and prints "Sender: sent 'Ping'".
Create a \`Receiver\` program that receives a message from \`messageChan\` into a variable (e.g. \`msg\`) and prints "Receiver: received " followed by that variable (e.g. \`print("Receiver: received ", msg);\`).
Run both programs.
  `,
  defaultCode: `// Declare channel messageChan for strings
// channel messageChan: string;

program Sender {
    // send "Ping" to messageChan;
    // print("Sender: sent 'Ping'");
}

program Receiver {
    // let msg = recv messageChan;
    // print("Receiver: received ", msg);
}

main {
    // run Sender;
    // run Receiver;
}`,
  validate: (code: string) => {
    const declaresChannel = /channel\\s+messageChan\\s*:\\s*string\\s*;/.test(code);
    const senderSendsAndPrints = /program\\s+Sender\\s*{[^}]*send\\s+"Ping"\\s+to\\s+messageChan;[^}]*print\\("Sender: sent 'Ping'"\\);[^}]*}/s.test(code);
    const receiverReceivesAndPrints = /program\\s+Receiver\\s*{[^}]*let\\s+(\\w+)\\s*=\\s*recv\\s+messageChan;[^}]*print\\("Receiver: received ",\\s*\\1\\);[^}]*}/s.test(code);
    const runsBothInMain = /main\\s*{[\\s\\S]*(run\\s+Sender;[\\s\\S]*run\\s+Receiver;|run\\s+Receiver;[\\s\\S]*run\\s+Sender;)[\s\\S]*}/s.test(code);

    if (declaresChannel && senderSendsAndPrints && receiverReceivesAndPrints && runsBothInMain) {
        return { success: true, message: "Channels used correctly for send/receive!" };
    }
    let issues = [];
    if (!declaresChannel) issues.push("declaration of 'messageChan: string'");
    if (!senderSendsAndPrints) issues.push("Sender program sending 'Ping' to messageChan and printing 'Sender: sent \\'Ping\\''");
    if (!receiverReceivesAndPrints) issues.push("Receiver program receiving from messageChan into a variable (e.g., 'msg'), and printing 'Receiver: received ' followed by that variable (e.g., 'print(\"Receiver: received \", msg);')");
    if (!runsBothInMain) issues.push("running both Sender and Receiver programs in the main block");
    return { success: false, message: `Please review your channel implementation: ${issues.join(', ')}.` };
  }
};
