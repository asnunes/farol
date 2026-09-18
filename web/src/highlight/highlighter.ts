import {
  createHighlighterCore,
  type GrammarState,
  type HighlighterCore,
} from "shiki/core";
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
let built: HighlighterCore | null = null;
const loaded = new Map<string, Promise<void>>();
const tokenizers = new Map<string, Tokenize>();

/** The highlighter once it exists, and a promise for it before that.
 *
 * `built` is filled in as part of building, not by a callback hung off the
 * promise afterwards. Hanging one on means the first render after a grammar
 * arrives still finds nothing — which showed up exactly as the code appearing
 * plain until the reader moved to another file and came back. */
function highlighter(): Promise<HighlighterCore> {
  core ??= createHighlighterCore({
    langs: [],
    themes: [light, dark],
    engine: createJavaScriptRegexEngine(),
  }).then((made) => (built = made));
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
  const engine = built;
  if (!engine || !engine.getLoadedLanguages().includes(id)) return null;

  const existing = tokenizers.get(id);
  if (existing) return existing;

  const options = {
    lang: id,
    themes: { light: light.name!, dark: dark.name! },
    defaultColor: false as const,
  };
  const convert = (
    tokens: ReturnType<typeof engine.codeToTokens>["tokens"],
  ): Token[][] =>
    tokens.map((line) =>
      line.map((token) => ({ content: token.content, style: token.htmlStyle })),
    );
  const tokenize: Tokenize = (code) =>
    convert(engine.codeToTokens(code, options).tokens);

  // Keep grammar checkpoints, not highlighted text for every file ever read.
  // The line array belongs to its hunk; releasing the diff also releases these.
  const checkpoints = new WeakMap<
    string[],
    Map<number, GrammarState | undefined>
  >();
  tokenize.range = (lines, from, to) => {
    let states = checkpoints.get(lines);
    if (!states) {
      states = new Map([[0, undefined]]);
      checkpoints.set(lines, states);
    }
    const start = Math.floor(from / CHECKPOINT_LINES) * CHECKPOINT_LINES;
    let cursor = start;
    while (!states.has(cursor)) cursor -= CHECKPOINT_LINES;
    while (cursor < start) {
      const next = Math.min(cursor + CHECKPOINT_LINES, lines.length);
      const result = engine.codeToTokens(lines.slice(cursor, next).join("\n"), {
        ...options,
        grammarState: states.get(cursor),
      });
      states.set(next, result.grammarState);
      cursor = next;
    }
    const result = engine.codeToTokens(lines.slice(start, to).join("\n"), {
      ...options,
      grammarState: states.get(start),
    });
    return convert(result.tokens).slice(from - start, to - start);
  };
  tokenizers.set(id, tokenize);
  return tokenize;
}

const CHECKPOINT_LINES = 80;
