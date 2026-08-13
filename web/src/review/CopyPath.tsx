import { Check, Copy } from "lucide-react";
import { useCopy } from "@/hooks/useCopy";

/** Copy a path to the clipboard, for pasting into a terminal or a message. */
export function CopyPath({ path }: CopyPathProps) {
  const { copied, copy } = useCopy();

  return (
    <button
      className="copypath grid size-6 shrink-0 cursor-pointer place-items-center rounded text-faint transition-colors hover:bg-sunken hover:text-accent focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
      onClick={() => void copy(path)}
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

type CopyPathProps = { path: string };
