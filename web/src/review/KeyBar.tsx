/** The shortcuts, always visible, so the keyboard does not have to be
 * discovered. */
export function KeyBar() {
  return (
    <nav className="keybar col-span-full flex gap-5 border-t border-rule bg-surface px-5 py-1.5 font-sans text-xs text-muted">
      {[
        [["j", "k", "↑", "↓"], "file"],
        [["n"], "next unread"],
        [[";", "↵"], "mark read"],
        [["[", "]", "⇞", "⇟"], "block"],
        [["?"], "help"],
      ].map(([keys, what], i) => (
        <span key={i}>
          {(keys as string[]).map((k) => (
            <Key key={k}>{k}</Key>
          ))}
          {what as string}
        </span>
      ))}
    </nav>
  );
}

function Key({ children }: KeyProps) {
  return (
    <kbd className="mr-1 rounded border border-rule-strong bg-sunken px-1.5 py-0.5 font-mono text-[0.6875rem] text-ink-soft">
      {children}
    </kbd>
  );
}

type KeyProps = { children: React.ReactNode };
