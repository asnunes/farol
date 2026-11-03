import { LensSection } from "@/review/lenses/LensSection";
import type { LensActions } from "@/hooks/useComments";
import type { OpenLenses } from "@/hooks/useOpenLenses";
import type { CitationView, LensView } from "@/api";

/** A block's lenses, after its context and before its code. */
export function BlockLenses({ lenses, onCite, onSeen, comments, folds }: BlockLensesProps) {
  return (
    <>
      {lenses.map((lens) => (
        <LensSection key={lens.id} lens={lens} onCite={onCite} onSeen={onSeen} comments={comments} folds={folds} />
      ))}
    </>
  );
}

type BlockLensesProps = {
  lenses: LensView[];
  onCite: (citation: CitationView) => void;
  onSeen: (lens: LensView, seen: boolean) => void;
  comments: LensActions;
  folds: OpenLenses;
};
