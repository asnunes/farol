import { ArrowDown, ArrowUp, Space } from "lucide-react";
import { Kbd } from "@/components/ui/kbd";
import { MOD } from "@/hooks/useShortcuts";
import { ThemeSwitch } from "@/review/ThemeSwitch";
import type { Theme } from "@/hooks/useTheme";

/** The shortcuts, always visible, so the keyboard does not have to be
 * discovered.
 *
 * A key with a letter or a symbol on it is written; a key that only has a name
 * is drawn. The characters for those, `↵` and `⇞`, come out of a monospace font
 * as two indistinguishable ticks at this size.
 *
 * The keys of the reading sit together on the right. `⌘B` sits on the left,
 * beside the theme, because it is the same kind of thing those two are: what
 * the screen looks like, rather than where the reader is in the review. In the
 * run on the right it read as one more way to move, and was lost among five
 * chips it does not belong to. */
export function KeyBar({ theme, onTheme }: KeyBarProps) {
  return (
    <nav className="keybar col-span-full flex items-center justify-between border-t border-rule bg-surface px-5 py-1.5 font-sans text-xs text-ink-muted">
      <div className="chrome flex items-center gap-4">
        <ThemeSwitch theme={theme} onChange={onTheme} />
        <Shortcut keys={[`${MOD}B`]} what="map" />
      </div>

      <div className="flex gap-5">
        {SHORTCUTS.map((shortcut) => (
          <Shortcut key={shortcut.what} {...shortcut} />
        ))}
      </div>
    </nav>
  );
}

function Shortcut({ keys, what }: ShortcutProps) {
  return (
    // Centred rather than sitting on a baseline: a chip holding an icon
    // and a chip holding a letter have different baselines inside them, and
    // a row aligned that way comes out stepped.
    <span className="flex items-center gap-1">
      {keys.map((key, i) => (
        <Key key={i}>{typeof key === "string" ? key : <key.Icon className="size-3" />}</Key>
      ))}
      <span className="ml-0.5">{what}</span>
    </span>
  );
}

function Key({ children }: KeyProps) {
  return (
    // `min-w` with padding rather than a fixed square: every other key on the
    // bar is one character, and `Ctrl+B` is six.
    <Kbd className="h-[1.35rem] min-w-[1.35rem] justify-center rounded border border-rule-strong bg-sunken px-1 text-[0.6875rem] text-ink-soft">
      {children}
    </Kbd>
  );
}

/** Moving, marking and asking: the keys the reading itself is done with. */
const SHORTCUTS: ShortcutProps[] = [
  { keys: ["j", "k", { Icon: ArrowUp }, { Icon: ArrowDown }], what: "file" },
  { keys: ["n"], what: "next unread" },
  { keys: [";", { Icon: Space }], what: "mark read" },
  { keys: ["[", "]"], what: "block" },
  { keys: ["?"], what: "help" },
];

type KeyBarProps = { theme: Theme; onTheme: (theme: Theme) => void };

type ShortcutProps = { keys: (string | { Icon: typeof ArrowUp })[]; what: string };

type KeyProps = { children: React.ReactNode };
