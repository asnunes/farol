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
 * `⌘B` opens the run rather than sitting inside it. It is the one key here
 * that is about the screen rather than about where the reader is in the
 * review, and buried between `block` and `help` that difference was invisible:
 * it read as one more way to move.
 *
 * It is also the one label that says what the key does rather than what it
 * moves through, and it changes with the sidebar, like the button's own
 * tooltip. It does not say *map*: the map is the whole review, which is what
 * the stale chip up in the top bar is counting commits against — the sidebar
 * is one way of looking at it. */
export function KeyBar({ theme, onTheme, sidebarOpen }: KeyBarProps) {
  const shortcuts = [
    { keys: [`${MOD}B`], what: sidebarOpen ? "hide sidebar" : "show sidebar" },
    ...SHORTCUTS,
  ];

  return (
    <nav className="keybar col-span-full flex items-center justify-between gap-3 border-t border-rule bg-surface px-3 py-1.5 font-sans text-xs text-ink-muted md:px-5">
      <ThemeSwitch theme={theme} onChange={onTheme} />

      {/* The run scrolls sideways where the bar is too narrow to hold it. These
          are a hint and not a control — wrapping them would spend three lines
          of a short screen on chrome, and dropping them would take the keys
          away from the reader on a narrow window who has a keyboard. */}
      <div className="keys flex min-w-0 gap-4 overflow-x-auto md:gap-5">
        {shortcuts.map(({ keys, what }) => (
          // Centred rather than sitting on a baseline: a chip holding an icon
          // and a chip holding a letter have different baselines inside them,
          // and a row aligned that way comes out stepped.
          <span key={what} className="flex shrink-0 items-center gap-1 whitespace-nowrap">
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

type KeyBarProps = { theme: Theme; onTheme: (theme: Theme) => void; sidebarOpen: boolean };

type KeyProps = { children: React.ReactNode };
