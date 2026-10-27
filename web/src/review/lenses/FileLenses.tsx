import { useState } from "react";
import { Eye, EyeOff } from "lucide-react";
import { Button } from "@/components/ui/button";
import { LensState } from "@/review/lenses/LensState";
import { LensText } from "@/review/lenses/LensText";
import type { CitationView, FileView, LensView } from "@/api";

/** A file's own lenses, under its header and before its diff, held to the file
 * by a rail under its name. They stay with the diff folded: they are what lets
 * the reader leave it folded. */
export function FileLenses({ file, onCite }: FileLensesProps) {
  if (file.lenses.length === 0 && !file.readYourself) return null;
  return (
    <div className="file-lenses border-b border-rule bg-surface pr-4 pb-2 pl-[4.25rem] md:pr-6">
      {file.readYourself && (
        <p className="read-yourself py-1.5 font-sans text-xs text-highlight">
          Read this yourself ({file.readYourself})
        </p>
      )}
      {file.lenses.map((lens) => (
        <FileLens key={lens.id} lens={lens} path={file.path} onCite={onCite} />
      ))}
    </div>
  );
}

function FileLens({ lens, path, onCite }: FileLensProps) {
  const [shown, setShown] = useState(true);
  return (
    <div className="file-lens mt-1.5 border-l-2 border-rule-strong pl-3.5" data-lens={lens.id}>
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
  onCite: (citation: CitationView) => void;
};

type FileLensProps = {
  lens: LensView;
  path: string;
  onCite: (citation: CitationView) => void;
};
