import { LensSection } from "@/review/lenses/LensSection";
import type { CitationView, LensView } from "@/api";

/** The pull request's lenses, before the first block: what the change is,
 * where its risk is and what was checked, read before any code. */
export function ReviewLenses({ lenses, onCite, onSeen }: ReviewLensesProps) {
  if (lenses.length === 0) return null;
  return (
    <div className="review-lenses">
      <div className="kicker border-b border-rule bg-surface px-4 py-3 font-mono text-[0.6875rem] tracking-wide text-ink-muted uppercase md:px-6">
        pull request
      </div>
      {lenses.map((lens) => (
        <LensSection key={lens.id} lens={lens} where={`pr:${lens.id}`} onCite={onCite} onSeen={onSeen} />
      ))}
    </div>
  );
}

type ReviewLensesProps = {
  lenses: LensView[];
  onCite: (citation: CitationView) => void;
  onSeen: (lens: LensView, seen: boolean) => void;
};
