import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { splitPath } from "@/lib/path";
import type { FileView } from "@/api";

/** One file in the sidebar: read state, name, and what is waiting inside it. */
export function FileRow({
  file,
  current,
  onPick,
}: FileRowProps) {
  return (
    <li>
      <Button
        variant="ghost"
        className={cn(
          "fileitem h-auto w-full justify-start gap-2 px-2 py-1 font-mono text-[0.8125rem] font-normal text-ink-soft",
          "hover:bg-sunken hover:text-ink-soft",
          "aria-[current=true]:bg-highlight-dim aria-[current=true]:text-ink",
          file.skim && "skim text-faint",
        )}
        data-seen={file.viewed ? "true" : undefined}
        aria-current={file.path === current ? "true" : undefined}
        title={file.skimReason ?? undefined}
        onClick={() => onPick(file.path)}
      >
        <span className="chk w-3 shrink-0 text-add-ink">{file.viewed ? "✓" : ""}</span>
        <span
          className={cn(
            "nm truncate",
            file.viewed && "text-faint line-through",
            file.path === current && "font-semibold",
          )}
        >
          {splitPath(file.path).name}
        </span>

        {file.skim ? (
          <Badge
            variant="outline"
            className="fast ml-auto shrink-0 border-rule px-1 font-mono text-[0.625rem] font-normal text-faint"
          >
            skim
          </Badge>
        ) : (
          file.lineNotes.length > 0 && (
            <span
              className="dot ml-auto shrink-0 text-highlight"
              title={`${file.lineNotes.length} note(s)`}
            >
              {"•".repeat(Math.min(3, file.lineNotes.length))}
            </span>
          )
        )}
      </Button>
    </li>
  );
}

type FileRowProps = {
  file: FileView;
  current: string | null;
  onPick: (path: string) => void;
};
