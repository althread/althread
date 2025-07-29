import { styleTags, tags as t } from "@lezer/highlight";
import { ProgName } from "./parser.terms";

export const althreadHighlight = styleTags({
  // Control and keywords
  "atomic": t.modifier,
  "while await if else receive send run for loop in return break continue": t.controlKeyword,
  "instanceof as": t.operatorKeyword,
  "let const channel": t.definitionKeyword,
  "program always eventually main shared fn import": t.moduleKeyword,

  "PrivateDirective": t.special(t.keyword),
  "PrivateDirective/private": t.keyword,
  
  // Function highlighting
  "FunctionBlock/fn": t.keyword,
  "FunctionBlock/FnName": t.function(t.definition(t.variableName)),
  "CallExpression/FnName": t.function(t.variableName),
  "FnName": t.function(t.variableName),

  "RunExprression/run": t.function(t.keyword),
  "RunExpression/ProgName": t.function(t.variableName),
  "ProgName": t.function(t.variableName),

  "MemberExpression > identifier:first-child": t.namespace,
  "CallExpression/MemberExpression > identifier:last-child": t.function(t.variableName),
  "MemberExpression > identifier:last-child": t.propertyName,


  "MemberExpression/object": t.namespace,
  "CallExpression/MemberExpression/property": t.function(t.variableName),
  "MemberExpression/property": t.propertyName,

  // Import highlighting
  "ImportBlock/import": t.moduleKeyword,
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
  Type: t.typeName,
  
  "Variable/self": t.self,
  LocalVariableName: t.variableName,
  SharedVariableName: t.propertyName,
  ProgName: t.className,
  
  // Operators and punctuation
  UpdateOp: t.updateOperator,
  "LineComment": t.lineComment,
  "BlockComment": t.blockComment,
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