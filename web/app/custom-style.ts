
import { tags as t } from '@lezer/highlight';

export default [
    { tag: t.propertyName, color: "#ff7979" },
    { tag: t.variableName, color: "#c4c4c4;" },
    { tag: t.macroName, color: "#739777" },
    { tag: t.string, color: "#9dd575" },
    { tag: t.moduleKeyword, color: "#8b4de4", fontWeight: "bold" },
    { tag: t.className, color: "#e5c07b", fontWeight: "bold" },
];