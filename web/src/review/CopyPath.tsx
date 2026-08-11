import { useRef, useState } from "react";
import { Check, Copy } from "lucide-react";

/** How long the tick stays up: long enough to be seen, short enough that it is
 * gone before anyone wonders whether it is about the copy they just made. */
const CONFIRM_FOR = 1500;

/** Copy a path to the clipboard, for pasting into a terminal or a message.
 *
 * The tick appears only after the write came back: confirming a copy that did
 * not happen is worse than not confirming one that did, because the paste is
 * what finds out. */
export function CopyPath({ path }: { path: string }) {
  const [copied, setCopied] = useState(false);
  const clearing = useRef<number | undefined>(undefined);

  async function copy() {
    try {
      await navigator.clipboard.writeText(path);
    } catch {
      return;
    }
    setCopied(true);
    window.clearTimeout(clearing.current);
    clearing.current = window.setTimeout(() => setCopied(false), CONFIRM_FOR);
  }

  return (
    <button
      className="copypath grid size-6 shrink-0 cursor-pointer place-items-center rounded text-faint transition-colors hover:bg-sunken hover:text-accent focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
      onClick={() => void copy()}
      aria-label={copied ? "Path copied" : "Copy path"}
      title="Copy path"
    >
      {copied ? (
        <Check className="size-3.5 text-add-ink" aria-hidden="true" />
      ) : (
        <Copy className="size-3.5" aria-hidden="true" />
      )}
    </button>
  );
}
