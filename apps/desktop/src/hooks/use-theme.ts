import { useCallback, useEffect, useState } from "react";

export type ThemePreference = "light" | "dark" | "system";
export type ResolvedTheme = "light" | "dark";

export const THEME_STORAGE_KEY = "skillspp.theme";

/**
 * Resolve a user preference to a concrete theme.
 * Pure function — semantics mirror the inline FOUC script in index.html.
 */
export function resolveTheme(preference: ThemePreference): ResolvedTheme {
  if (preference === "system") {
    return window.matchMedia("(prefers-color-scheme: light)").matches
      ? "light"
      : "dark";
  }
  return preference;
}

function readPreference(): ThemePreference {
  try {
    const stored = localStorage.getItem(THEME_STORAGE_KEY);
    if (stored === "light" || stored === "dark" || stored === "system") {
      return stored;
    }
  } catch {
    /* ignore */
  }
  return "system";
}

async function applyThemeToWindow(resolved: ResolvedTheme) {
  try {
    // Lazy import so web-only test environments don't choke on Tauri APIs.
    const mod = await import("@tauri-apps/api/window");
    const win = mod.getCurrentWindow();
    // Tauri expects "light" | "dark" | null; null means follow system.
    await win.setTheme(resolved);
  } catch {
    /* not running under Tauri — ignore */
  }
}

function applyTheme(preference: ThemePreference) {
  const resolved = resolveTheme(preference);
  document.documentElement.dataset.theme = resolved;
  void applyThemeToWindow(resolved);
}

export function useTheme() {
  const [preference, setPreferenceState] = useState<ThemePreference>(() =>
    readPreference(),
  );

  const setPreference = useCallback((next: ThemePreference) => {
    try {
      localStorage.setItem(THEME_STORAGE_KEY, next);
    } catch {
      /* ignore */
    }
    setPreferenceState(next);
    applyTheme(next);
  }, []);

  // Apply once on mount so the DOM and native window agree with state
  // (the inline FOUC script already set dataset.theme before paint).
  useEffect(() => {
    applyTheme(preference);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // When following system, react to OS appearance changes in real time.
  useEffect(() => {
    if (preference !== "system") return;
    const mql = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => applyTheme("system");
    mql.addEventListener("change", onChange);
    return () => mql.removeEventListener("change", onChange);
  }, [preference]);

  const resolved = resolveTheme(preference);

  return { preference, resolved, setPreference } as const;
}
