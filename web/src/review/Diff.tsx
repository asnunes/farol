import { useMemo } from "react";
import { cn } from "@/lib/utils";
import { colourHunk, type Token, type Tokenize } from "@/highlight/tokens";
import { useHighlight } from "@/hooks/useHighlight";
import type { FileDiff, FileView, Hunk } from "@/api";

/** The code itself, with the session's line notes beside the lines they are
 * about. */
export function Diff({ diff, file }: DiffProps) {
  const tokenize = useHighlight(diff.binary ? null : diff.path);

  if (diff.binary) {
    // Nothing to read line by line, so say that rather than show an empty pane
    // the reviewer would take for a loading failure.
    return (
      <div className="diff nodiff px-6 py-6 font-sans text-sm text-muted">
        Binary file — not shown
      </div>
    );
  }

  return (
    <div className="diff pb-24 font-mono text-[0.8125rem] leading-relaxed">
      {diff.hunks.map((hunk, i) => (
        <DiffHunk key={i} hunk={hunk} file={file} tokenize={tokenize} />
      ))}
    </div>
  );
}

function DiffHunk({
  hunk,
  file,
  tokenize,
}: DiffHunkProps) {
  // Both sides of the hunk go through the tokenizer once, not once per render:
  // navigation redraws this on every keystroke.
  const coloured = useMemo(
    () => (tokenize ? colourHunk(hunk, tokenize) : null),
    [hunk, tokenize],
  );

  return (
    <div>
      <div className="hunk bg-sunken px-6 py-1 text-xs text-faint">
        @@ -{hunk.old_start},{hunk.old_lines} +{hunk.new_start},{hunk.new_lines} @@
      </div>
      {hunk.lines.map((line, i) => {
        // A note is anchored to the last line of its span, which is where the
        // reader has finished reading the thing it is about.
        const notes = file.lineNotes.filter(
          (n) => line.new_number !== null && n.to === line.new_number,
        );
        // The lines the note is about, marked so the span is visible before the
        // note explains it. The range printed on the note says which lines it
        // covers, but nobody counts line numbers to find them.
        const noted = file.lineNotes.some(
          (n) => line.new_number !== null && n.from <= line.new_number && line.new_number <= n.to,
        );
        const marker = line.kind === "added" ? "+" : line.kind === "removed" ? "−" : " ";

        return (
          <div key={i}>
            <div
              className={cn(
                "row diff-row",
                line.kind === "added" && "add bg-add-bg text-add-ink",
                line.kind === "removed" && "del bg-del-bg text-del-ink",
                noted && "noted",
              )}
            >
              <div className="ln shrink-0 pr-3 text-right text-faint select-none">
                {line.new_number ?? line.old_number ?? ""}
              </div>
              <div className="code overflow-x-auto whitespace-pre">
                {marker} <Code tokens={coloured?.[i]} plain={line.content} />
              </div>
            </div>
            {notes.map((n, j) => (
              <div
                key={j}
                className="note noted border-b border-note-rule bg-note-bg py-2 pr-6 pl-16 font-serif text-[0.9375rem] leading-relaxed text-ink-soft"
              >
                <span className="lbl mr-2 font-mono text-xs text-accent">
                  {n.from}–{n.to}
                  {file.tags.length > 1 ? ` · ${n.block}` : ""}
                </span>
                {n.text}
              </div>
            ))}
          </div>
        );
      })}
    </div>
  );
}

/** The line, coloured if its grammar has arrived and plain until then. The
 * palette for both themes rides on the token as custom properties, so the
 * stylesheet decides which one applies and nothing is tokenized twice. */
function Code({ tokens, plain }: CodeProps) {
  if (!tokens) return <>{plain}</>;

  return (
    <>
      {tokens.map((token, i) => (
        <span key={i} className="tok" style={token.style}>
          {token.content}
        </span>
      ))}
    </>
  );
}

type DiffProps = { diff: FileDiff; file: FileView };

type DiffHunkProps = {
  hunk: Hunk;
  file: FileView;
  tokenize: Tokenize | null;
};

type CodeProps = { tokens?: Token[]; plain: string };
