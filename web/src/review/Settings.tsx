import { useState } from "react";
import { Settings as SettingsIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { useConfig } from "@/hooks/useConfig";

export function Settings() {
  const [open, setOpen] = useState(false);
  const { config, error, saving, saved, setOpenBrowser } = useConfig(open);

  if (!config?.show_open_browser) return null;

  return (
    <>
      <Button
        variant="ghost"
        size="icon-xs"
        className="text-ink-muted hover:bg-transparent hover:text-ink max-md:size-9"
        onClick={() => setOpen(true)}
        aria-label="Settings"
        title="Settings"
      >
        <SettingsIcon className="size-3.5" aria-hidden="true" />
      </Button>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="border-rule bg-surface sm:max-w-md">
          <DialogHeader>
            <DialogTitle className="font-sans text-base text-ink">Settings</DialogTitle>
            <DialogDescription className="text-ink-muted">
              For all repositories on this machine, under your user account.
            </DialogDescription>
          </DialogHeader>
          {config ? (
            <label className="flex items-start gap-3 text-sm text-ink">
              <input type="checkbox" className="mt-1 size-4 accent-current" checked={config.open_browser} disabled={saving} onChange={(event) => void setOpenBrowser(event.target.checked)} aria-describedby="open-browser-description" />
              <span>
                Open browser automatically
                <span id="open-browser-description" className="mt-1 block text-xs leading-relaxed text-ink-muted">
                  Applies the next time you run Farol.
                </span>
              </span>
            </label>
          ) : !error && <p role="status" className="text-sm text-ink-muted">Loading settings…</p>}
          {error && <p role="alert" className="text-sm text-ink">{error}</p>}
          <p role="status" className="min-h-4 text-xs text-ink-muted">{saving ? "Saving…" : saved ? "Saved" : ""}</p>
        </DialogContent>
      </Dialog>
    </>
  );
}
