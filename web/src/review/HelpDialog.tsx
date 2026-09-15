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

export function HelpDialog({
  open,
  onOpenChange,
}: HelpDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="help-card border-rule bg-surface sm:max-w-sm">
        <DialogHeader>
          <DialogTitle className="font-sans text-base text-ink">Keys</DialogTitle>
        </DialogHeader>
        <dl className="grid grid-cols-[5rem_minmax(0,1fr)] gap-x-4 gap-y-2 text-sm">
          {KEYS.map(([key, what]) => (
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
      </DialogContent>
    </Dialog>
  );
}

type HelpDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
};
