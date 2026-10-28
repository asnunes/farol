import { Checkbox } from "@/components/ui/checkbox";
import type { LensView } from "@/api";

/** The tick that marks a lens as seen, the same tick a file has. Only on a
 * lens with something written: there is nothing to have seen otherwise. */
export function SeenBox({ lens, onSeen }: SeenBoxProps) {
  if (!lens.target) return null;
  return (
    <Checkbox
      className="seenbox size-4 border-rule-strong hover:border-highlight data-[state=checked]:border-highlight data-[state=checked]:bg-highlight data-[state=checked]:text-surface"
      checked={lens.seen}
      onCheckedChange={(checked) => onSeen(lens, checked === true)}
      aria-label={`Mark ${lens.title} as seen`}
      title="Mark as seen"
    />
  );
}

type SeenBoxProps = {
  lens: LensView;
  onSeen: (lens: LensView, seen: boolean) => void;
};
