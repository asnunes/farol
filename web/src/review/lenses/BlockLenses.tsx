import { LensSection } from "@/review/lenses/LensSection";
import type { CitationView, LensView } from "@/api";

/** A block's lenses, after its context and before its code. */
export function BlockLenses({ slug, lenses, onCite, onSeen }: BlockLensesProps) {
  return (
    <>
      {lenses.map((lens) => (
        <LensSection key={lens.id} lens={lens} where={`block:${slug}:${lens.id}`} onCite={onCite} onSeen={onSeen} />
      ))}
    </>
  );
}

type BlockLensesProps = {
  slug: string;
  lenses: LensView[];
  onCite: (citation: CitationView) => void;
  onSeen: (lens: LensView, seen: boolean) => void;
};
