import { useMemo } from "react";
import { cn } from "@/lib/utils";
import { colourHunk, type Token, type Tokenize } from "@/highlight/tokens";
import { useHighlight } from "@/hooks/useHighlight";
import { splitRows } from "@/review/split";
import type { DiffView } from "@/hooks/useDiffView";
import type { DiffLine, FileDiff, FileView, Hunk } from "@/api";

/** The code itself, with the session's line notes beside the lines they are
 * about. */
export function Diff({ diff, file, view }: DiffProps) {
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
        <DiffHunk key={i} hunk={hunk} file={file} tokenize={tokenize} view={view} />
      ))}
    </div>
  );
}

function DiffHunk({ hunk, file, tokenize, view }: DiffHunkProps) {
  // Both sides of the hunk go through the tokenizer once, not once per render:
  // navigation redraws this on every keystroke.
  const coloured = useMemo(
    () => (tokenize ? colourHunk(hunk, tokenize) : null),
    [hunk, tokenize],
  );
  const rows = useMemo(() => splitRows(hunk.lines), [hunk]);

  /** The lines a note is about, so the span is visible before the note
   * explains it. The range printed on the note says which lines it covers, but
   * nobody counts line numbers to find them. */
  const noted = (line: DiffLine) =>
    file.lineNotes.some(
      (n) => line.new_number !== null && n.from <= line.new_number && line.new_number <= n.to,
    );

  /** A note is anchored to the last line of its span, which is where the reader
   * has finished reading the thing it is about. */
  const notesAt = (line: DiffLine) =>
    file.lineNotes.filter((n) => line.new_number !== null && n.to === line.new_number);

  return (
    <div>
      <div className="hunk bg-sunken px-6 py-1 text-xs text-faint">
        @@ -{hunk.old_start},{hunk.old_lines} +{hunk.new_start},{hunk.new_lines} @@
      </div>

      {view === "split"
        ? rows.map((row, i) => {
            const anchor = row.right ?? row.left;
            const line = anchor === null ? null : hunk.lines[anchor];

            return (
              <div key={i}>
                <div
                  className={cn("row split-row", line && noted(line) && "noted")}
                >
                  <Side index={row.left} hunk={hunk} coloured={coloured} side="old" />
                  <Side index={row.right} hunk={hunk} coloured={coloured} side="new" />
                </div>
                {line && <Notes notes={notesAt(line)} file={file} />}
              </div>
            );
          })
        : hunk.lines.map((line, i) => (
            <div key={i}>
              <div
                className={cn(
                  "row diff-row",
                  line.kind === "added" && "add bg-add-bg text-add-ink",
                  line.kind === "removed" && "del bg-del-bg text-del-ink",
                  noted(line) && "noted",
                )}
              >
                <div className="ln shrink-0 pr-3 text-right text-faint select-none">
                  {line.new_number ?? line.old_number ?? ""}
                </div>
                {/* Wraps instead of scrolling sideways: a narrow window would
                    otherwise cut the line off, and reading code by dragging a
                    horizontal bar is worse than reading it on two lines. */}
                <div className="code break-words whitespace-pre-wrap">
                  {marker(line)} <Code tokens={coloured?.[i]} plain={line.content} />
                </div>
              </div>
              <Notes notes={notesAt(line)} file={file} />
            </div>
          ))}
    </div>
  );
}

/** One side of a split row: its number and its code, or an empty pair where the
 * other side has a line and this one does not. The two cells are separate grid
 * children so the columns line up across every row of the hunk. */
function Side({ index, hunk, coloured, side }: SideProps) {
  if (index === null) {
    return (
      <>
        <div className="ln absent bg-sunken" />
        <div className="code absent bg-sunken" />
      </>
    );
  }

  const line = hunk.lines[index];
  const tint =
    line.kind === "added"
      ? "add bg-add-bg text-add-ink"
      : line.kind === "removed"
        ? "del bg-del-bg text-del-ink"
        : "";

  return (
    <>
      <div className={cn("ln shrink-0 pr-3 text-right text-faint select-none", tint)}>
        {side === "old" ? line.old_number : line.new_number}
      </div>
      <div className={cn("code break-words whitespace-pre-wrap", tint)}>
        {marker(line)} <Code tokens={coloured?.[index]} plain={line.content} />
      </div>
    </>
  );
}

function Notes({ notes, file }: NotesProps) {
  return notes.map((n, i) => (
    <div
      key={i}
      className="note noted border-b border-note-rule bg-note-bg py-2 pr-6 pl-16 font-serif text-[0.9375rem] leading-relaxed text-ink-soft"
    >
      <span className="lbl mr-2 font-mono text-xs text-accent">
        {n.from}–{n.to}
        {file.tags.length > 1 ? ` · ${n.block}` : ""}
      </span>
      {n.text}
    </div>
  ));
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

function marker(line: DiffLine) {
  return line.kind === "added" ? "+" : line.kind === "removed" ? "−" : " ";
}

type DiffProps = { diff: FileDiff; file: FileView; view: DiffView };

type DiffHunkProps = {
  hunk: Hunk;
  file: FileView;
  tokenize: Tokenize | null;
  view: DiffView;
};

type SideProps = {
  index: number | null;
  hunk: Hunk;
  coloured: Token[][] | null;
  side: "old" | "new";
};

type NotesProps = { notes: FileView["lineNotes"]; file: FileView };

type CodeProps = { tokens?: Token[]; plain: string };
