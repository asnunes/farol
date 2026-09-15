import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import type { FileLabel } from "@/lib/path";
import type { FileView } from "@/api";

/** One file in the sidebar: read state, name, and what is waiting inside it.
 *
 * The name leads and the directory follows only when something else on screen
 * is called the same thing — the whole path in every row would be a column of
 * near-identical prefixes to read past. The path is always a keystroke away in
 * the tooltip, and is what a screen reader announces. */
export function FileRow({
  file,
  label,
  comments,
  current,
  onPick,
}: FileRowProps) {
  return (
    <li>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant="ghost"
            className={cn(
              "fileitem h-auto w-full justify-start gap-2 px-2 py-1 font-mono text-[0.8125rem] font-normal text-ink-soft",
              "hover:bg-sunken hover:text-ink-soft",
              "aria-[current=true]:bg-highlight-dim aria-[current=true]:text-ink",
              file.skim && "skim text-ink-muted",
            )}
            data-seen={file.viewed ? "true" : undefined}
            aria-current={file.path === current ? "true" : undefined}
            aria-label={file.path}
            onClick={() => onPick(file.path)}
          >
            <span className="chk w-3 shrink-0 text-add-ink">{file.viewed ? "✓" : ""}</span>
            <span
              className={cn(
                "nm truncate",
                file.viewed && "text-ink-muted line-through",
                file.path === current && "font-semibold",
              )}
            >
              {label.name}
            </span>
            {label.where && (
              <span aria-hidden="true" className="where truncate text-[0.6875rem] text-ink-muted">
                · {label.where}
              </span>
            )}

            {file.skim && (
              <Badge
                variant="outline"
                className="fast ml-auto shrink-0 border-rule px-1 font-mono text-[0.625rem] font-normal text-ink-muted"
              >
                skim
              </Badge>
            )}

            {/* Blue because that is the reader's own voice here, the same one
                the comment wears down in the diff. A number rather than a run
                of marks: a count that stops counting is one to distrust. */}
            {comments > 0 && (
              <span
                className={cn(
                  "asked grid h-4 min-w-4 shrink-0 place-items-center rounded-full",
                  "bg-comment-dim px-1 font-mono text-[0.625rem] text-comment-ink",
                  !file.skim && "ml-auto",
                )}
              >
                {comments}
              </span>
            )}
          </Button>
        </TooltipTrigger>
        {/* A real tooltip rather than `title`: the browser's own waits a second,
            never opens on keyboard focus, and would drop the skim reason that
            used to be the only thing it carried. */}
        <TooltipContent className="max-w-[28rem]">
          <div className="font-mono text-[0.6875rem]">{file.path}</div>
          {file.skimReason && <div className="font-sans opacity-80">{file.skimReason}</div>}
          {/* The button is labelled with the path, which is what a screen
              reader reads instead of the row — so the count only reaches one
              from here. */}
          {comments > 0 && (
            <div className="font-sans opacity-80">
              {comments === 1 ? "1 comment" : `${comments} comments`}
            </div>
          )}
        </TooltipContent>
      </Tooltip>
    </li>
  );
}

type FileRowProps = {
  file: FileView;
  label: FileLabel;
  /** How many comments the reader has left open on this file. */
  comments: number;
  current: string | null;
  onPick: (path: string) => void;
};
