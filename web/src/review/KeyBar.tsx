import { ArrowDown, ArrowUp, Space } from "lucide-react";
import { Kbd } from "@/components/ui/kbd";
import { ThemeSwitch } from "@/review/ThemeSwitch";
import type { Theme } from "@/hooks/useTheme";

/** The shortcuts, always visible, so the keyboard does not have to be
 * discovered.
 *
 * A key with a letter or a symbol on it is written; a key that only has a name
 * is drawn. The characters for those, `↵` and `⇞`, come out of a monospace font
 * as two indistinguishable ticks at this size. */
export function KeyBar({ theme, onTheme }: KeyBarProps) {
  return (
    <nav className="keybar col-span-full flex items-center justify-between border-t border-rule bg-surface px-5 py-1.5 font-sans text-xs text-muted">
      <ThemeSwitch theme={theme} onChange={onTheme} />

      <div className="flex gap-5">
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
      </div>
    </nav>
  );
}

function Key({ children }: KeyProps) {
  return (
    <Kbd className="size-[1.35rem] justify-center rounded border border-rule-strong bg-sunken text-[0.6875rem] text-ink-soft">
      {children}
    </Kbd>
  );
}

const SHORTCUTS: { keys: (string | { Icon: typeof ArrowUp })[]; what: string }[] = [
  { keys: ["j", "k", { Icon: ArrowUp }, { Icon: ArrowDown }], what: "file" },
  { keys: ["n"], what: "next unread" },
  { keys: [";", { Icon: Space }], what: "mark read" },
  { keys: ["[", "]"], what: "block" },
  { keys: ["?"], what: "help" },
];

type KeyBarProps = { theme: Theme; onTheme: (theme: Theme) => void };

type KeyProps = { children: React.ReactNode };
