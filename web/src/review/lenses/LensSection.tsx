import { ChevronDown, ChevronRight } from "lucide-react";
import { Button } from "@/components/ui/button";
import { LensState } from "@/review/lenses/LensState";
import { LensText } from "@/review/lenses/LensText";
import { SeenBox } from "@/review/lenses/SeenBox";
import { ProseThreads } from "@/review/ProseThreads";
import type { CommentActions } from "@/hooks/useComments";
import type { OpenLenses } from "@/hooks/useOpenLenses";
import type { CitationView, LensView } from "@/api";

/** One lens of the pull request or of a block: open, in the flow of the
 * reading, and folded away with its chevron the way a file is. */
export function LensSection({ lens, where, onCite, onSeen, comments, folds }: LensSectionProps) {
  // Seen folds it away, like a file marked read, unless the reader said
  // otherwise against the mark it has now.
  const open = folds.isOpen(where, lens.seen);
  const setOpen = (next: boolean) => folds.set(where, lens.seen, next);

  return (
    <section className="lens border-b border-rule bg-ground px-4 py-3 md:px-6" data-lens={lens.id} data-lens-key={where} data-done={lens.seen ? "true" : undefined}>
      <div className="flex items-center gap-2">
        <Button
          variant="ghost"
          size="icon-xs"
          className="size-5 text-faint hover:bg-transparent hover:text-ink"
          aria-expanded={open}
          aria-label={open ? `Collapse ${lens.title}` : `Expand ${lens.title}`}
          onClick={() => setOpen(!open)}
        >
          {open ? <ChevronDown className="size-3.5" aria-hidden="true" /> : <ChevronRight className="size-3.5" aria-hidden="true" />}
        </Button>
        <SeenBox lens={lens} onSeen={onSeen} />
        <h3 className={`font-sans text-sm font-semibold ${lens.seen ? "text-ink-muted" : "text-ink"}`}>{lens.title}</h3>
        <LensState lens={lens} />
      </div>
      {open &&
        (lens.state === "missing" ? (
          <p className="mt-1 pl-7 font-sans text-sm text-ink-muted">Not written yet — written on the next map update.</p>
        ) : (
          <>
            <LensText
              className={`mt-1 max-w-[72ch] pl-7 font-serif text-sm leading-relaxed text-ink-soft ${lens.state === "outdated" ? "opacity-70" : ""}`}
              text={lens.text}
              citations={lens.citations}
              onCite={onCite}
            />
          </>
        ))}
      {/* Outside the fold: a lens folded away once seen, or not written yet,
          still shows its threads, so none is left drawn nowhere. A new one is
          started only on a lens open with something written. */}
      <div className="pl-7">
        <ProseThreads
          about={open && lens.target ? { lens: lens.id, target: lens.target } : null}
          label={`Thread on the ${lens.title} lens`}
          threads={comments.onLens(where)}
          actions={comments}
        />
      </div>
    </section>
  );
}

type LensSectionProps = {
  lens: LensView;
  /** Which lens on which target, as the sidebar finds it. */
  where: string;
  onCite: (citation: CitationView) => void;
  onSeen: (lens: LensView, seen: boolean) => void;
  folds: OpenLenses;
  comments: CommentActions;
};
