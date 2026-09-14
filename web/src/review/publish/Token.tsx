import { useId, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { said } from "@/lib/utils";

/** Where the token goes in. Masked, and an input rather than the textarea this
 * was: a credential sitting in plain text is a credential handed to whoever is
 * watching the screen share, and a review is a thing people screen-share.
 *
 * The textarea existed so a ninety-character fine-grained token could be read
 * back whole after a paste. That check is what Show token is for — the reader
 * asks for the value at the moment they want to check it, instead of the box
 * offering it to the room for the whole time the panel is open. */
export function Token({ host, onToken }: TokenProps) {
  const field = useId();
  const [token, setToken] = useState("");
  const [shown, setShown] = useState(false);
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
      // Back to masked with it: leaving the box revealed would show the next
      // token typed into it from the first keystroke.
      setShown(false);
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
      <div className="mb-1.5 flex items-center justify-between gap-2">
        <label htmlFor={field} className="font-sans text-xs font-medium text-ink-soft">
          GitHub token
        </label>
        {/* The label is the whole state: it says what pressing does, which is
            also what the box is not doing now. An `aria-pressed` beside a name
            that already changes would announce the same fact twice. */}
        <Button
          size="xs"
          variant="ghost"
          className="text-faint hover:text-ink"
          onClick={() => setShown(!shown)}
        >
          {shown ? "Hide token" : "Show token"}
        </Button>
      </div>
      <Input
        id={field}
        type={shown ? "text" : "password"}
        className="border-rule-strong bg-surface font-mono text-xs text-ink"
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
