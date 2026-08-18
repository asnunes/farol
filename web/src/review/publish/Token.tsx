import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";

/** Where the token goes in. A textarea and not an input: a fine-grained token
 * runs to ninety characters, and a single line hides all but the tail of it
 * exactly when the reader wants to check they pasted the whole thing.
 *
 * `break-all` is what makes that true. A token has no spaces in it, so soft
 * wrapping treats the whole thing as one word and runs it off the side, which
 * is the very problem the textarea was chosen to avoid. */
export function Token({ onToken }: { onToken: (token: string) => Promise<void> }) {
  const [token, setToken] = useState("");
  const [saving, setSaving] = useState(false);

  async function save() {
    if (!token.trim() || saving) return;
    setSaving(true);
    try {
      await onToken(token);
      // Out of the page as soon as it is out of the box. It is not coming back
      // from the server, and there is no reason for it to sit in a form.
      setToken("");
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="token">
      <Textarea
        className="resize-none break-all border-rule-strong bg-surface font-mono text-xs text-ink"
        rows={3}
        placeholder="github_pat_…"
        autoComplete="off"
        spellCheck={false}
        value={token}
        disabled={saving}
        onChange={(e) => setToken(e.target.value)}
      />
      <Button
        size="sm"
        className="mt-2 cursor-pointer"
        disabled={!token.trim() || saving}
        onClick={() => void save()}
      >
        {saving ? "Saving…" : "Save token"}
      </Button>
    </div>
  );
}
