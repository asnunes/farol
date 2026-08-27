import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import type { Verdict } from "@/api";

/** What the review says about the change as a whole, as three buttons.
 *
 * `approveOnly` narrows it to one: comment and request changes are somebody
 * being asked for something, and a review with nothing waiting to go is not
 * asking for anything. */
export function Verdicts({ value, onChange, approveOnly }: VerdictsProps) {
  return (
    <ToggleGroup
      type="single"
      className="verdicts justify-start gap-2"
      value={value}
      onValueChange={(next) => next && onChange(next as Verdict)}
    >
      {(approveOnly ? APPROVE_ONLY : ALL).map(([verdict, label, why]) => (
        <ToggleGroupItem
          key={verdict}
          value={verdict}
          title={why}
          className="rounded border border-rule px-3 font-sans text-xs text-ink-soft hover:bg-sunken hover:text-ink data-[state=on]:bg-accent-dim data-[state=on]:text-accent"
        >
          {label}
        </ToggleGroupItem>
      ))}
    </ToggleGroup>
  );
}

/** The three the API takes, in the order a reviewer works through them. */
const ALL: [Verdict, string, string][] = [
  ["comment", "Comment", "Leave the comments without a verdict"],
  ["requestChanges", "Request changes", "Ask for the change to be reworked"],
  ["approve", "Approve", "Say it is good to merge"],
];

const APPROVE_ONLY = ALL.filter(([verdict]) => verdict === "approve");

type VerdictsProps = {
  value: Verdict;
  onChange: (verdict: Verdict) => void;
  approveOnly: boolean;
};
