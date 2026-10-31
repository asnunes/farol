import { LensSection } from "@/review/lenses/LensSection";
import type { CommentActions } from "@/hooks/useComments";
import type { OpenLenses } from "@/hooks/useOpenLenses";
import type { CitationView, LensView } from "@/api";

/** A block's lenses, after its context and before its code. */
export function BlockLenses({ slug, lenses, onCite, onSeen, comments, folds }: BlockLensesProps) {
  return (
    <>
      {lenses.map((lens) => (
        <LensSection key={lens.id} lens={lens} where={`block:${slug}:${lens.id}`} onCite={onCite} onSeen={onSeen} comments={comments} folds={folds} />
      ))}
    </>
  );
}

type BlockLensesProps = {
  slug: string;
  lenses: LensView[];
  onCite: (citation: CitationView) => void;
  onSeen: (lens: LensView, seen: boolean) => void;
  comments: CommentActions;
  folds: OpenLenses;
};
