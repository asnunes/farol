import { Moon, Sun } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { Theme } from "@/hooks/useTheme";

/** Light or dark, from the bar the reader already looks at for the keys. */
export function ThemeSwitch({ theme, onChange }: ThemeSwitchProps) {
  const next = theme === "dark" ? "light" : "dark";

  return (
    <Button
      variant="ghost"
      size="icon-xs"
      className="themeswitch text-ink-muted hover:bg-transparent hover:text-ink max-md:size-9"
      onClick={() => onChange(next)}
      aria-label={`Switch to the ${next} theme`}
      title={`Switch to the ${next} theme`}
    >
      {theme === "dark" ? (
        <Sun className="size-3.5" aria-hidden="true" />
      ) : (
        <Moon className="size-3.5" aria-hidden="true" />
      )}
    </Button>
  );
}

type ThemeSwitchProps = { theme: Theme; onChange: (theme: Theme) => void };
