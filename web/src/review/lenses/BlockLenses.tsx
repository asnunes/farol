import { LensSection } from "@/review/lenses/LensSection";
import type { CitationView, LensView } from "@/api";

/** A block's lenses, after its context and before its code. */
export function BlockLenses({ lenses, onCite }: BlockLensesProps) {
  return (
    <>
      {lenses.map((lens) => (
        <LensSection key={lens.id} lens={lens} onCite={onCite} />
      ))}
    </>
  );
}

type BlockLensesProps = {
  lenses: LensView[];
  onCite: (citation: CitationView) => void;
};
