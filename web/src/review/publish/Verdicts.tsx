import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import type { Verdict } from "@/api";

/** What the review says about the change as a whole, as three buttons.
 *
 * On your own pull request there is only one: GitHub takes a comment there and
 * refuses the other two, so they are not drawn. A picker with a single item is
 * not a picker, so that case says the verdict in words instead of leaving a
 * button that cannot be pressed or unpressed. */
export function Verdicts({ value, onChange, mine }: VerdictsProps) {
  if (mine) {
    return (
      <p className="verdicts font-serif text-sm text-ink-muted">
        This pull request is yours, so it goes up as a comment. GitHub keeps
        approving and asking for changes for somebody else.
      </p>
    );
  }

  return (
    <ToggleGroup
      type="single"
      className="verdicts justify-start gap-2"
      value={value}
      onValueChange={(next) => next && onChange(next as Verdict)}
    >
      {ALL.map(([verdict, label, why]) => (
        <ToggleGroupItem
          key={verdict}
          value={verdict}
          title={why}
          className="rounded border border-rule px-3 font-sans text-xs text-ink-soft hover:bg-sunken hover:text-ink data-[state=on]:bg-highlight-dim data-[state=on]:text-highlight"
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

type VerdictsProps = {
  value: Verdict;
  onChange: (verdict: Verdict) => void;
  /** The reviewer opened this pull request themselves. */
  mine: boolean;
};
