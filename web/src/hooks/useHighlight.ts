import { useEffect, useState } from "react";
import { isBundled, load, tokenizerFor } from "@/highlight/highlighter";
import { languageOf } from "@/highlight/language";
import type { Tokenize } from "@/highlight/tokens";

/** The tokenizer for the file on screen, once its grammar has arrived.
 *
 * Returns nothing until then, and nothing at all for a file whose language
 * farol cannot name — both of which the diff draws the same way, plain. The
 * page is never held up waiting for colour. */
export function useHighlight(path: string | null): Tokenize | null {
  const [ready, setReady] = useState(0);
  const language = path ? languageOf(path, isBundled) : null;

  useEffect(() => {
    if (!language) return;

    let watching = true;
    void load(language).then(() => {
      // A grammar that arrives after the reader has moved on is not worth a
      // repaint, and setting state on a gone component is how that shows up.
      if (watching) setReady((n) => n + 1);
    });
    return () => {
      watching = false;
    };
  }, [language]);

  // `ready` is not read: it exists to make the arrival of a grammar a render.
  void ready;
  return language ? tokenizerFor(language) : null;
}
