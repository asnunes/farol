import { Check, Copy } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useCopy } from "@/hooks/useCopy";

/** Copy a path to the clipboard, for pasting into a terminal or a message. */
export function CopyPath({ path }: CopyPathProps) {
  const { copied, copy } = useCopy();

  return (
    <Button
      variant="ghost"
      size="icon-xs"
      className="copypath text-faint hover:bg-sunken hover:text-accent"
      onClick={() => void copy(path)}
      aria-label={copied ? "Path copied" : "Copy path"}
      title="Copy path"
    >
      {copied ? (
        <Check className="size-3.5 text-add-ink" aria-hidden="true" />
      ) : (
        <Copy className="size-3.5" aria-hidden="true" />
      )}
    </Button>
  );
}

type CopyPathProps = { path: string };
