/** Which grammar a path is written in.
 *
 * Shiki indexes by language name and a diff carries file names, so something
 * has to translate. The table only holds the extensions whose name is not the
 * language: `.go`, `.css` and `.php` already are, and are left to the fallback
 * rather than written out twice. */
const BY_EXTENSION: Record<string, string> = {
  rs: "rust",
  ts: "typescript",
  mts: "typescript",
  cts: "typescript",
  js: "javascript",
  mjs: "javascript",
  cjs: "javascript",
  py: "python",
  rb: "ruby",
  kt: "kotlin",
  kts: "kotlin",
  cs: "csharp",
  cc: "cpp",
  cxx: "cpp",
  hpp: "cpp",
  h: "c",
  md: "markdown",
  markdown: "markdown",
  sh: "shellscript",
  bash: "shellscript",
  zsh: "shellscript",
  fish: "fish",
  yml: "yaml",
  htm: "html",
  ex: "elixir",
  exs: "elixir",
  erl: "erlang",
  hs: "haskell",
  pl: "perl",
  ps1: "powershell",
  tf: "hcl",
  tfvars: "hcl",
  gql: "graphql",
  vim: "viml",
  el: "emacs-lisp",
  clj: "clojure",
  cljs: "clojure",
};

/** Files whose whole name is the clue. */
const BY_NAME: Record<string, string> = {
  dockerfile: "dockerfile",
  makefile: "make",
  justfile: "make",
  gemfile: "ruby",
  rakefile: "ruby",
  "cargo.lock": "toml",
  "go.sum": "text",
  ".gitignore": "ignore",
  ".gitattributes": "ini",
  ".env": "dotenv",
};

/** The language, or nothing — and nothing is a normal answer. A file farol
 * cannot name is drawn the way every file was drawn before any of this: plain,
 * with no error and nothing missing from the screen. */
export function languageOf(path: string, known: (id: string) => boolean): string | null {
  const file = path.split("/").pop()?.toLowerCase() ?? "";

  const byName = BY_NAME[file];
  if (byName) return known(byName) ? byName : null;

  const extension = file.includes(".") ? file.split(".").pop()! : "";
  if (!extension) return null;

  // The table, then the extension itself: `go`, `css`, `json`, `toml`, `lua`,
  // `swift` and a long tail besides are already the name of their grammar, and
  // listing them would be a second place to forget one.
  const guess = BY_EXTENSION[extension] ?? extension;
  return known(guess) ? guess : null;
}
