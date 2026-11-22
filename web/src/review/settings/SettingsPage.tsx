import { useEffect } from "react";
import { X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { useConfig } from "@/hooks/useConfig";
import { useInstructionDrafts } from "@/hooks/useInstructionDrafts";
import { useLensDrafts } from "@/hooks/useLensDrafts";
import { useLenses } from "@/hooks/useLenses";
import { SECTIONS, type SettingsSection } from "@/hooks/useSettingsRoute";
import { GeneralSection } from "./GeneralSection";
import { InstructionsSection } from "./InstructionsSection";
import { LensesSection } from "./LensesSection";
import { SaveBar } from "./SaveBar";
import type { BooleanSetting, Setting, TextSetting } from "@/api";

/** Settings, over the whole page: a section per kind of preference, picked
 * from the side, with what is being edited held until the section's Save.
 *
 * Over the review rather than in place of it: the review stays mounted
 * underneath, so closing Settings puts the reader back exactly where they
 * were. It outgrew the dialog it started as once the lens catalog moved in. */
export function SettingsPage({ section, onSection, onClose, guard }: SettingsPageProps) {
  const config = useConfig(true);
  const lenses = useLenses(true);
  const settings = config.config?.settings ?? [];
  const instructions = useInstructionDrafts(settings.filter(isText), {
    set: (key, value) => config.setSetting(key, value),
    reset: (key) => config.resetSetting(key),
  });
  const lensDrafts = useLensDrafts(lenses.lenses, lenses);
  const unsaved = instructions.count + lensDrafts.count;
  const saving = config.saving || lenses.saving;

  // Read by whoever closes Settings, which is the address as much as the X.
  guard.current = () => unsaved === 0 || window.confirm("You have unsaved changes. Leave anyway?");

  useEffect(() => {
    if (unsaved === 0) return;
    const hold = (e: BeforeUnloadEvent) => e.preventDefault();
    window.addEventListener("beforeunload", hold);
    return () => window.removeEventListener("beforeunload", hold);
  }, [unsaved]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  const drafts = section === "instructions" ? instructions : section === "lenses" ? lensDrafts : null;

  return (
    <div role="dialog" aria-modal="true" aria-labelledby="settings-title" className="settings fixed inset-0 z-50 flex flex-col bg-ground">
      <header className="flex items-start justify-between gap-4 border-b border-rule bg-surface px-4 py-3 md:px-6">
        <div>
          <h1 id="settings-title" className="font-sans text-base font-semibold text-ink">Settings</h1>
          <p className="font-sans text-xs text-ink-muted">For all repositories on this machine, under your user account.</p>
        </div>
        <Button variant="ghost" size="icon-xs" className="text-ink-muted hover:text-ink" aria-label="Close settings" title="Close (Esc)" onClick={onClose}>
          <X className="size-4" aria-hidden="true" />
        </Button>
      </header>

      <div className="flex min-h-0 flex-1 flex-col md:flex-row">
        {/* A column on the side, and a row of tabs where there is no room. */}
        <nav aria-label="Settings sections" className="flex shrink-0 gap-1 border-b border-rule bg-surface px-2 py-2 md:w-52 md:flex-col md:border-r md:border-b-0 md:py-4">
          {SECTIONS.map((one) => (
            <Button
              key={one}
              variant="ghost"
              size="sm"
              aria-current={one === section ? "page" : undefined}
              className={cn(
                "justify-start font-sans text-sm text-ink-muted hover:bg-sunken hover:text-ink",
                one === section && "bg-sunken text-ink",
              )}
              onClick={() => onSection(one)}
            >
              {NAMES[one]}
            </Button>
          ))}
        </nav>

        <div className="settings-body relative flex min-h-0 flex-1 flex-col">
          <div className="min-h-0 flex-1 overflow-y-auto px-4 py-6 md:px-8">
            <div className="mx-auto flex max-w-3xl flex-col gap-6">
              <h2 className="font-sans text-lg font-semibold text-ink">{NAMES[section]}</h2>
              {section === "general" && (
                <GeneralSection
                  switches={settings.filter(isSwitch)}
                  saving={saving}
                  onSet={(key, value) => void config.setSetting(key, value)}
                />
              )}
              {section === "instructions" && <InstructionsSection settings={settings.filter(isText)} drafts={instructions} saving={saving} />}
              {section === "lenses" && <LensesSection lenses={lenses} drafts={lensDrafts} />}
              {config.error && section === "general" && <p role="alert" className="text-sm text-del-ink">{config.error}</p>}
            </div>
          </div>
          {drafts && drafts.count > 0 && (
            <SaveBar count={drafts.count} saving={saving} onSave={() => void drafts.save()} onDiscard={drafts.discard} />
          )}
        </div>
      </div>
    </div>
  );
}

const NAMES: Record<SettingsSection, string> = {
  general: "General",
  instructions: "Instructions",
  lenses: "Lenses",
};

function isText(setting: Setting): setting is TextSetting {
  return setting.type === "text";
}

function isSwitch(setting: Setting): setting is BooleanSetting {
  return setting.type === "boolean";
}

type SettingsPageProps = {
  section: SettingsSection;
  onSection: (section: SettingsSection) => void;
  onClose: () => void;
  /** Set here, asked by whoever closes Settings: may it go with what is unsaved? */
  guard: React.RefObject<() => boolean>;
};
