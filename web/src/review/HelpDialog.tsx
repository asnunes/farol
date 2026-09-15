import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Kbd, KbdGroup } from "@/components/ui/kbd";
import { MOD } from "@/hooks/useShortcuts";

const KEYS: [string, string][] = [
  ["j / k", "previous and next file, in reading order"],
  ["↑ / ↓", "the same"],
  ["n", "jump to the next file you have not read"],
  [";", "mark the current file read"],
  ["Space", "the same"],
  ["[ / ]", "previous and next block"],
  [`${MOD}B`, "show or hide the sidebar"],
  ["?", "this list"],
];

/** The keys that only mean anything while the `+` in the gutter has focus, so
 * they are listed apart from the ones that work anywhere. Tab reaches the `+`
 * on a line; these are what turn the one line it offers into a passage. */
const ON_THE_PLUS: [string, string][] = [
  ["shift ↑ / shift ↓", "reach for another line, up or down"],
  ["Enter", "write the comment, over every line reached"],
];

export function HelpDialog({
  open,
  onOpenChange,
}: HelpDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="help-card border-rule bg-surface sm:max-w-md">
        <DialogHeader>
          <DialogTitle className="font-sans text-base text-ink">Keys</DialogTitle>
        </DialogHeader>
        <Keys of={KEYS} />
        <h3 className="font-sans text-xs font-medium tracking-wide text-ink-muted uppercase">
          On the + in the gutter
        </h3>
        <Keys of={ON_THE_PLUS} />
      </DialogContent>
    </Dialog>
  );
}

/** One run of keys and what they do. */
function Keys({ of }: KeysProps) {
  return (
    <dl className="grid grid-cols-[7.5rem_minmax(0,1fr)] gap-x-4 gap-y-2 text-sm">
      {of.map(([key, what]) => (
        <div key={key} className="contents">
          <dt className="text-ink">
            <KbdGroup>
              {key.split(" / ").map((one) => (
                <Kbd key={one} className="border-rule-strong bg-sunken text-ink-soft">
                  {one}
                </Kbd>
              ))}
            </KbdGroup>
          </dt>
          <dd className="font-serif text-ink-soft">{what}</dd>
        </div>
      ))}
    </dl>
  );
}

type HelpDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
};

type KeysProps = { of: [string, string][] };
