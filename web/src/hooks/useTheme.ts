import { useEffect, useState } from "react";

/** Light or dark, the reader's choice, remembered between visits.
 *
 * Starts wherever the system is and stays wherever it is put. The choice rides
 * on the root element, which is what every `light-dark` in the stylesheet is
 * reading. */
export function useTheme(): [Theme, (theme: Theme) => void] {
  const [theme, setTheme] = useState<Theme>(remembered);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    try {
      localStorage.setItem(KEY, theme);
    } catch {
      // Storage can be refused, and a preference that fails to persist is not
      // worth breaking the screen over.
    }
  }, [theme]);

  return [theme, setTheme];
}

export type Theme = "light" | "dark";

const KEY = "farol:theme";

function remembered(): Theme {
  try {
    const kept = localStorage.getItem(KEY);
    if (kept === "light" || kept === "dark") return kept;
  } catch {
    // Same as above: fall through to what the system says.
  }
  // jsdom has no matchMedia, and neither does a browser old enough to lack it.
  const dark = typeof matchMedia === "function" && matchMedia("(prefers-color-scheme: dark)").matches;
  return dark ? "dark" : "light";
}
