import { styleTags, tags as t } from "@lezer/highlight";
import { ProgName } from "./parser.terms";

export const althreadHighlight = styleTags({
	// Control and keywords
	"AtomicStatement/atomic AtomicStatement/@": t.modifier,
	"WhileStatement/while WaitStatement/await IfStatement/if IfStatement/else ReceiveExpression/receive SendExpression/send RunExpression/run ForStatement/for LoopStatement/loop ForStatement/in ReturnStatement/return BreakLoopStatement/break BreakLoopStatement/continue UnaryExpression/void":
		t.controlKeyword,
	"ImportItem/as": t.operatorKeyword,
	"VariableDeclaration/let VariableDeclaration/const SharedDeclaration/let SharedDeclaration/const ChannelDeclarationStatement/channel":
		t.definitionKeyword,

	// LTL / Formula keywords (different color)
	"LtlUnaryExpression/eventually LtlUnaryExpression/always LtlBinaryExpression/until":
		t.special(t.keyword),
	"LtlIfExpression/if LtlIfExpression/else LtlForExpression/for LtlForExpression/in":
		t.special(t.keyword),

	// Block declaration keywords (Bolder/Distinct)
	"MainBlock/main ProgramBlock/program GlobalBlock/shared ImportBlock/import FunctionBlock/fn ConditionBlock/Condition/always ConditionBlock/Condition/check ConditionBlock/Condition/never":
		t.moduleKeyword,

	PrivateDirective: t.moduleKeyword,
	"PrivateDirective/private": t.moduleKeyword,

	// Function highlighting
	"FunctionBlock/FnName": t.function(t.definition(t.variableName)),
	"CallExpression/FnName": t.function(t.variableName),
	FnName: t.function(t.variableName),

	"RunExpression/ProgName": t.function(t.variableName),
	ProgName: t.function(t.variableName),

	"MemberExpression > identifier:first-child": t.namespace,
	"CallExpression/MemberExpression > identifier:last-child": t.function(
		t.variableName,
	),
	"MemberExpression > identifier:last-child": t.propertyName,

	"MemberExpression/object": t.namespace,
	"CallExpression/MemberExpression/property": t.function(t.variableName),
	"MemberExpression/property": t.propertyName,

	// Import highlighting
	"ImportBlock/as": t.operatorKeyword,
	"ImportBlock/ImportList/ImportItem/ImportPath": t.namespace,
	"ImportBlock/ImportList/ImportItem/ImportAlias": t.namespace,
	ImportPath: t.namespace,
	ImportAlias: t.namespace,
	ImportSlash: t.separator,

	TemplateString: t.special(t.string),
	super: t.atom,
	BooleanLiteral: t.bool,
	this: t.self,
	null: t.null,
	Star: t.modifier,
	TypeToken: t.typeName,

	"Variable/self": t.self,
	LocalVariableName: t.variableName,
	SharedVariableName: t.propertyName,
	ProgName: t.className,

	// Operators and punctuation
	UpdateOp: t.updateOperator,
	LineComment: t.lineComment,
	BlockComment: t.blockComment,
	Number: t.number,
	String: t.string,
	ArithOp: t.arithmeticOperator,
	LogicOp: t.logicOperator,
	BitOp: t.bitwiseOperator,
	CompareOp: t.compareOperator,
	RegExp: t.regexp,
	Equals: t.definitionOperator,
	Arrow: t.function(t.punctuation),
	"( )": t.paren,
	"[ ]": t.squareBracket,
	"{ }": t.brace,
	"InterpolationStart InterpolationEnd": t.special(t.brace),
	".": t.derefOperator,
	", ;": t.separator,
});
