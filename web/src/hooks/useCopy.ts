import { useRef, useState } from "react";

/** How long the tick stays up: long enough to be seen, short enough that it is
 * gone before anyone wonders whether it is about the copy they just made. */
const CONFIRM_FOR = 1500;

/** Putting something on the clipboard, and saying so afterwards.
 *
 * The tick appears only after the write came back: confirming a copy that did
 * not happen is worse than not confirming one that did, because the paste is
 * what finds out. */
export function useCopy() {
  const [copied, setCopied] = useState(false);
  const clearing = useRef<number | undefined>(undefined);

  async function copy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      return;
    }
    setCopied(true);
    window.clearTimeout(clearing.current);
    clearing.current = window.setTimeout(() => setCopied(false), CONFIRM_FOR);
  }

  return { copied, copy };
}
