import { styleTags, tags as t } from "@lezer/highlight";
import { ProgName } from "./parser.terms";

export const althreadHighlight = styleTags({
  "atomic": t.modifier,
  "while wait if else receive send run":
    t.controlKeyword,
  "instanceof": t.operatorKeyword,
  "let const": t.definitionKeyword,
  "program always main shared": t.moduleKeyword,
  "with debugger as new": t.keyword,
  TemplateString: t.special(t.string),
  super: t.atom,
  BooleanLiteral: t.bool,
  this: t.self,
  null: t.null,
  Star: t.modifier,
  Type: t.typeName,

  LocalVariableName: t.variableName,
  SharedVariableName: t.propertyName,
  ProgName: t.className,
  FnName: t.macroName,

  
  PropertyDefinition: t.definition(t.propertyName),
  PrivatePropertyDefinition: t.definition(t.special(t.propertyName)),
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
  ": Spread": t.punctuation,
  "( )": t.paren,
  "[ ]": t.squareBracket,
  "{ }": t.brace,
  "InterpolationStart InterpolationEnd": t.special(t.brace),
  ".": t.derefOperator,
  ", ;": t.separator,
});