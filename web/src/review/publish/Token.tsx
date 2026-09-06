import { useState } from "react";
import { Button } from "@/components/ui/button";
import { said } from "@/lib/utils";
import { Textarea } from "@/components/ui/textarea";

/** Where the token goes in. A textarea and not an input: a fine-grained token
 * runs to ninety characters, and a single line hides all but the tail of it
 * exactly when the reader wants to check they pasted the whole thing.
 *
 * `break-all` is what makes that true. A token has no spaces in it, so soft
 * wrapping treats the whole thing as one word and runs it off the side, which
 * is the very problem the textarea was chosen to avoid. */
export function Token({ host, onToken }: TokenProps) {
  const [token, setToken] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function save() {
    if (!token.trim() || saving) return;
    setSaving(true);
    setError(null);
    try {
      await onToken(host, token);
      // Out of the page as soon as it is out of the box. It is not coming back
      // from the server, and there is no reason for it to sit in a form.
      setToken("");
    } catch (error) {
      setError(said(error));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="token">
      <p className="mb-2 text-sm text-ink">
        Authorize <strong>{host}</strong> to receive this token. Only continue if you trust this host.
      </p>
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
      {error && <p role="alert" className="mt-2 whitespace-pre-wrap text-sm text-ink">{error}</p>}
      <Button
        size="sm"
        className="mt-2"
        disabled={!token.trim() || saving}
        onClick={() => void save()}
      >
        {saving ? "Saving…" : `Save token for ${host}`}
      </Button>
    </div>
  );
}

type TokenProps = {
  host: string;
  onToken: (host: string, token: string) => Promise<void>;
};
