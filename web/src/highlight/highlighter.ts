import { createHighlighterCore, type HighlighterCore } from "shiki/core";
import { createJavaScriptRegexEngine } from "shiki/engine/javascript";
import { bundledLanguages } from "shiki/langs";
import { dark, light } from "./theme";
import type { Token, Tokenize } from "./tokens";

/** Shiki, loaded a grammar at a time.
 *
 * The regex engine in JavaScript rather than the WebAssembly one: it costs a
 * few exotic grammars and saves half a megabyte that every reader would carry
 * to read one language. Every grammar Shiki ships is reachable, and each is its
 * own file — the page starts with none of them and fetches the one the file in
 * front of you needs. */

let core: Promise<HighlighterCore> | null = null;
const loaded = new Map<string, Promise<void>>();

function highlighter(): Promise<HighlighterCore> {
  core ??= createHighlighterCore({
    langs: [],
    themes: [light, dark],
    engine: createJavaScriptRegexEngine(),
  });
  return core;
}

export function isBundled(id: string): boolean {
  return id in bundledLanguages;
}

/** Fetch the grammar for a language, once, however often it is asked for. */
export async function load(id: string): Promise<void> {
  if (!isBundled(id)) return;

  let pending = loaded.get(id);
  if (!pending) {
    pending = (async () => {
      const [engine, grammar] = await Promise.all([
        highlighter(),
        bundledLanguages[id as keyof typeof bundledLanguages](),
      ]);
      await engine.loadLanguage(grammar);
    })();
    loaded.set(id, pending);
  }
  await pending;
}

/** A tokenizer for a language already loaded, or nothing.
 *
 * Synchronous on purpose: the diff renders on every keystroke of navigation,
 * and awaiting per hunk would make the code arrive after the page. Loading is
 * the caller's business, and until it has finished the file is drawn plain. */
export function tokenizerFor(id: string): Tokenize | null {
  const engine = ready();
  if (!engine || !engine.getLoadedLanguages().includes(id)) return null;

  return (code: string) =>
    engine
      .codeToTokens(code, { lang: id, themes: { light: light.name!, dark: dark.name! }, defaultColor: false })
      .tokens.map((line) =>
        line.map((token): Token => ({ content: token.content, style: token.htmlStyle })),
      );
}

/** The highlighter if it has finished being built, without waiting for it. */
let built: HighlighterCore | null = null;
function ready(): HighlighterCore | null {
  if (!built && core) void core.then((h) => (built = h));
  return built;
}
