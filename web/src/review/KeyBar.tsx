import { ArrowDown, ArrowUp, Space } from "lucide-react";
import { Button } from "@/components/ui/button";
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
 * `⌘B` opens the run rather than sitting inside it. It is the one key here
 * that is about the screen rather than about where the reader is in the
 * review, and buried between `block` and `help` that difference was invisible:
 * it read as one more way to move.
 *
 * It is also the one label that says what the key does rather than what it
 * moves through, and it changes with the sidebar, like the button's own
 * tooltip. It does not say *map*: the map is the whole review, which is what
 * the stale chip up in the top bar is counting commits against — the sidebar
 * is one way of looking at it.
 *
 * Under `md` the run does not fit, and a row of chips cut off mid-word is worse
 * than no row at all: it looks broken and offers nothing to press. One button
 * stands in for the whole list, which is the only way to reach the keys at all
 * on a screen with no `?` to press. */
export function KeyBar({ theme, onTheme, sidebarOpen, onHelp }: KeyBarProps) {
  const shortcuts = [
    { keys: [`${MOD}B`], what: sidebarOpen ? "hide sidebar" : "show sidebar" },
    ...SHORTCUTS,
  ];

  return (
    <nav className="keybar col-span-full flex items-center justify-between gap-3 border-t border-rule bg-surface px-4 py-1.5 font-sans text-xs text-ink-muted md:px-5">
      <ThemeSwitch theme={theme} onChange={onTheme} />

      <Button
        variant="ghost"
        size="sm"
        className="keyhelp gap-1.5 font-sans text-xs font-normal text-ink-muted hover:bg-sunken hover:text-ink md:hidden"
        onClick={onHelp}
      >
        <Key>?</Key>
        keys
      </Button>

      <div className="keys hidden gap-5 md:flex">
        {shortcuts.map(({ keys, what }) => (
          // Centred rather than sitting on a baseline: a chip holding an icon
          // and a chip holding a letter have different baselines inside them,
          // and a row aligned that way comes out stepped.
          <span key={what} className="flex items-center gap-1 whitespace-nowrap">
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
    // `min-w` with padding rather than a fixed square: every other key on the
    // bar is one character, and `Ctrl+B` is six.
    <Kbd className="h-[1.35rem] min-w-[1.35rem] justify-center rounded border border-rule-strong bg-sunken px-1 text-[0.6875rem] text-ink-soft">
      {children}
    </Kbd>
  );
}

/** The keys of the reading itself, which say nothing about the screen. */
const SHORTCUTS: { keys: (string | { Icon: typeof ArrowUp })[]; what: string }[] = [
  { keys: ["j", "k", { Icon: ArrowUp }, { Icon: ArrowDown }], what: "file" },
  { keys: ["n"], what: "next unread" },
  { keys: [";", { Icon: Space }], what: "mark read" },
  { keys: ["[", "]"], what: "block" },
  { keys: ["?"], what: "help" },
];

type KeyBarProps = {
  theme: Theme;
  onTheme: (theme: Theme) => void;
  sidebarOpen: boolean;
  /** Open the list of keys, for the screen that has no room to print it. */
  onHelp: () => void;
};

type KeyProps = { children: React.ReactNode };
