import { Moon, Sun } from "lucide-react";
import type { Theme } from "@/hooks/useTheme";

/** Light or dark, from the bar the reader already looks at for the keys. */
export function ThemeSwitch({ theme, onChange }: ThemeSwitchProps) {
  const next = theme === "dark" ? "light" : "dark";

  return (
    <button
      className="themeswitch grid size-6 cursor-pointer place-items-center rounded text-muted transition-colors hover:text-ink focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
      onClick={() => onChange(next)}
      aria-label={`Switch to the ${next} theme`}
      title={`Switch to the ${next} theme`}
    >
      {theme === "dark" ? (
        <Sun className="size-3.5" aria-hidden="true" />
      ) : (
        <Moon className="size-3.5" aria-hidden="true" />
      )}
    </button>
  );
}

type ThemeSwitchProps = { theme: Theme; onChange: (theme: Theme) => void };
