import { useState } from "react";
import { ChevronDown, ChevronRight, Eye, EyeOff } from "lucide-react";
import { Button } from "@/components/ui/button";
import { LensState } from "@/review/lenses/LensState";
import { LensText } from "@/review/lenses/LensText";
import type { CitationView, FileView, LensView } from "@/api";

/** A file's own lenses, under its header and before its diff, held to the file
 * by a rail under its name, with why the map puts the file in focus when it
 * does, and the way into the code after them: the lenses are read first, and
 * they are what lets the reader leave the code folded.
 *
 * No tick of their own. They are about the file, so the file's tick is the one
 * that says they were read, and marking it folds them away with the code. */
export function FileLenses({ file, open, onToggleOpen, onCite }: FileLensesProps) {
  if (!speaksAbout(file) || (file.viewed && !open)) return null;
  return (
    <div className="file-lenses border-b border-rule bg-surface pr-4 pb-2 pl-[4.25rem] md:pr-6">
      {file.focus && (
        <p className="focus py-1.5 font-sans text-xs text-highlight">Focus: {file.focus}</p>
      )}
      {file.lenses.map((lens) => (
        <FileLens key={lens.id} lens={lens} path={file.path} onCite={onCite} />
      ))}
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

function FileLens({ lens, path, onCite }: FileLensProps) {
  const [shown, setShown] = useState(true);
  return (
    <div className="file-lens mt-1.5 border-l-2 border-rule-strong pl-3.5" data-lens={lens.id} data-lens-key={`file:${path}:${lens.id}`}>
      <div className="flex items-center gap-2">
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
      {shown && (
        <LensText
          className="font-serif text-[0.9rem] leading-relaxed text-ink-soft"
          text={lens.text}
          citations={lens.citations}
          inFile={path}
          onCite={onCite}
        />
      )}
    </div>
  );
}

type FileLensesProps = {
  file: FileView;
  open: boolean;
  onToggleOpen: () => void;
  onCite: (citation: CitationView) => void;
};

type FileLensProps = {
  lens: LensView;
  path: string;
  onCite: (citation: CitationView) => void;
};
