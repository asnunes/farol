import { useState } from "react";
import { ChevronDown, ChevronRight, Eye, EyeOff } from "lucide-react";
import { Button } from "@/components/ui/button";
import { LensState } from "@/review/lenses/LensState";
import { LensText } from "@/review/lenses/LensText";
import { ProseThreads } from "@/review/ProseThreads";
import type { CommentActions } from "@/hooks/useComments";
import type { CitationView, FileView, LensView } from "@/api";

/** A file's own lenses, under its header and before its diff, held to the file
 * by a rail under its name, with why the map puts the file in focus when it
 * does, and the way into the code after them: the lenses are read first, and
 * they are what lets the reader leave the code folded.
 *
 * No tick of their own. They are about the file, so the file's tick is the one
 * that says they were read, and marking it folds them away with the code. */
export function FileLenses({ file, open, onToggleOpen, onCite, comments }: FileLensesProps) {
  if (!speaksAbout(file) || (file.viewed && !open)) return null;
  return (
    <div className="file-lenses border-b border-rule bg-surface px-4 pt-0.5 pb-2 md:px-6">
      {file.focus && (
        <p className="focus py-1.5 font-sans text-xs text-highlight">Focus: {file.focus}</p>
      )}
      {/* One rail for all of them, with a dot at each title: a timeline
          down the file, read top to bottom before the code. */}
      {file.lenses.length > 0 && (
        <div className="timeline border-l-2 border-rule-strong">
          {file.lenses.map((lens) => (
            <FileLens key={lens.id} lens={lens} path={file.path} onCite={onCite} comments={comments} />
          ))}
        </div>
      )}
      {!file.viewed && (
        <Button
          variant="ghost"
          size="xs"
          className="showcode mt-2 -ml-2 gap-1 font-sans text-xs text-ink-muted hover:bg-transparent hover:text-ink"
          aria-expanded={open}
          onClick={onToggleOpen}
        >
          {open ? <ChevronDown className="size-3.5" aria-hidden="true" /> : <ChevronRight className="size-3.5" aria-hidden="true" />}
          {open ? "Hide code" : "Show code"}
        </Button>
      )}
    </div>
  );
}

/** Whether a file has lenses to draw, which is also whether the way into its
 * code sits under them instead of on the header. */
export function speaksAbout(file: FileView): boolean {
  return file.lenses.length > 0 || file.focus !== null;
}

function FileLens({ lens, path, onCite, comments }: FileLensProps) {
  const [shown, setShown] = useState(true);
  return (
    <div className="file-lens relative pt-1.5 pl-3.5" data-lens={lens.id} data-lens-key={`file:${path}:${lens.id}`}>
      <div className="flex items-center gap-2">
        <span aria-hidden="true" className="dot absolute top-3 -left-[5px] size-2 rounded-full bg-rule-strong" />
        <span className="font-mono text-[0.6875rem] tracking-wide text-ink-muted uppercase">{lens.title}</span>
        <LensState lens={lens} />
        <Button
          variant="ghost"
          size="icon-xs"
          className="ml-auto size-5 text-faint hover:bg-transparent hover:text-ink"
          aria-pressed={!shown}
          aria-label={shown ? `Hide ${lens.title}` : `Show ${lens.title}`}
          title={shown ? "Hide" : "Show"}
          onClick={() => setShown(!shown)}
        >
          {shown ? <Eye className="size-3.5" aria-hidden="true" /> : <EyeOff className="size-3.5" aria-hidden="true" />}
        </Button>
      </div>
      {lens.open && <p className="opens font-sans text-xs text-highlight">Open: {lens.open}</p>}
      {shown && (
        <LensText
          className="font-serif text-sm leading-relaxed text-ink-soft"
          text={lens.text}
          citations={lens.citations}
          inFile={path}
          onCite={onCite}
        />
      )}
      <ProseThreads
        about={shown && lens.target ? { lens: lens.id, target: lens.target } : null}
        label={`Thread on the ${lens.title} lens`}
        threads={comments.onLens(`file:${path}:${lens.id}`)}
        actions={comments}
      />
    </div>
  );
}

type FileLensesProps = {
  file: FileView;
  open: boolean;
  onToggleOpen: () => void;
  onCite: (citation: CitationView) => void;
  comments: CommentActions;
};

type FileLensProps = {
  lens: LensView;
  path: string;
  onCite: (citation: CitationView) => void;
  comments: CommentActions;
};
