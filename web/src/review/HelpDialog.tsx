import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

const KEYS: [string, string][] = [
  ["j / k", "previous and next file, in reading order"],
  ["n", "jump to the next file you have not read"],
  ["e", "mark the current file read"],
  ["[ / ]", "previous and next block"],
  ["?", "this list"],
];

type HelpDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
};

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
              <dt className="font-mono text-ink">{key}</dt>
              <dd className="font-serif text-ink-soft">{what}</dd>
            </div>
          ))}
        </dl>
      </DialogContent>
    </Dialog>
  );
}
