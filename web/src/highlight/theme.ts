import type { ThemeRegistrationRaw } from "shiki/core";

/** The six things syntax is allowed to say here. */
/** Syntax colour, on a screen where colour already means something else.
 *
 * Green and red are taken by added and removed, amber is the session's own
 * voice. What is left for syntax is blue, violet and the greys — so there are
 * five roles and no more, and punctuation is quieter than the text it sits in.
 * A theme lifted from an editor would put a seventh and eighth colour on a
 * screen whose whole argument is that colour carries meaning. */
function roles(c: Palette) {
  return [
    {
      scope: ["comment", "punctuation.definition.comment"],
      settings: { foreground: c.comment, fontStyle: "italic" },
    },
    {
      scope: [
        "keyword",
        "storage",
        "storage.type",
        "storage.modifier",
        "keyword.control",
        "keyword.operator.new",
      ],
      settings: { foreground: c.keyword },
    },
    {
      scope: [
        "string",
        "string.quoted",
        "constant.character",
        "constant.other.symbol",
        "constant.numeric",
        "constant.language",
        "constant.other",
      ],
      settings: { foreground: c.literal },
    },
    {
      scope: [
        "entity.name.type",
        "entity.name.class",
        "entity.name.namespace",
        "entity.name.function",
        "entity.name.tag",
        "entity.other.attribute-name",
        "entity.other.inherited-class",
        "support.type",
        "support.class",
        "support.function",
        "support.type.property-name",
        "meta.function-call",
      ],
      settings: { foreground: c.name },
    },
    {
      scope: ["variable", "meta.definition.variable", "entity.name.variable", "variable.parameter"],
      settings: { foreground: c.text },
    },
    {
      scope: ["punctuation", "meta.brace", "keyword.operator"],
      settings: { foreground: c.punctuation },
    },
  ];
}

export const light: ThemeRegistrationRaw = {
  name: "farol-light",
  type: "light",
  colors: { "editor.background": "#ffffff", "editor.foreground": "#16181b" },
  settings: roles({
    comment: "#949ba3",
    keyword: "#7d4fae",
    literal: "#26708f",
    name: "#2d5aa0",
    text: "#16181b",
    punctuation: "#697079",
  }),
};

export const dark: ThemeRegistrationRaw = {
  name: "farol-dark",
  type: "dark",
  colors: { "editor.background": "#1c1f23", "editor.foreground": "#e6e8ea" },
  settings: roles({
    comment: "#6c747d",
    keyword: "#c39ae8",
    literal: "#7fc3de",
    name: "#8fb4e8",
    text: "#e6e8ea",
    punctuation: "#8a929b",
  }),
};

type Palette = {
  comment: string;
  keyword: string;
  literal: string;
  name: string;
  text: string;
  punctuation: string;
};
