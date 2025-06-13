import { styleTags, tags as t } from "@lezer/highlight";
import { ProgName } from "./parser.terms";

export const althreadHighlight = styleTags({
  "atomic": t.modifier,
  "while await if else receive send run for loop in return break continue":
    t.controlKeyword,
  "instanceof": t.operatorKeyword,
  "let const channel": t.definitionKeyword,
  "program always eventually main shared fn": t.moduleKeyword,
  //"with as new": t.keyword,
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
  FnName: t.macroName,

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