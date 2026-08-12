import { ArrowDown, ArrowUp, ChevronsDown, ChevronsUp, CornerDownLeft } from "lucide-react";

/** The shortcuts, always visible, so the keyboard does not have to be
 * discovered.
 *
 * A key with a letter or a symbol on it is written; a key that only has a name
 * is drawn. The characters for those, `↵` and `⇞`, come out of a monospace font
 * as two indistinguishable ticks at this size. */
export function KeyBar() {
  return (
    <nav className="keybar col-span-full flex gap-5 border-t border-rule bg-surface px-5 py-1.5 font-sans text-xs text-muted">
      {SHORTCUTS.map(({ keys, what }) => (
        // Centred rather than sitting on a baseline: a chip holding an icon
        // and a chip holding a letter have different baselines inside them, and
        // a row aligned that way comes out stepped.
        <span key={what} className="flex items-center gap-1">
          {keys.map((key, i) => (
            <Key key={i}>{typeof key === "string" ? key : <key.Icon className="size-3" />}</Key>
          ))}
          <span className="ml-0.5">{what}</span>
        </span>
      ))}
    </nav>
  );
}

function Key({ children }: KeyProps) {
  return (
    <kbd className="grid size-[1.35rem] shrink-0 place-items-center rounded border border-rule-strong bg-sunken font-mono text-[0.6875rem] text-ink-soft">
      {children}
    </kbd>
  );
}

const SHORTCUTS: { keys: (string | { Icon: typeof ArrowUp })[]; what: string }[] = [
  { keys: ["j", "k", { Icon: ArrowUp }, { Icon: ArrowDown }], what: "file" },
  { keys: ["n"], what: "next unread" },
  { keys: [";", { Icon: CornerDownLeft }], what: "mark read" },
  { keys: ["[", "]", { Icon: ChevronsUp }, { Icon: ChevronsDown }], what: "block" },
  { keys: ["?"], what: "help" },
];

type KeyProps = { children: React.ReactNode };
