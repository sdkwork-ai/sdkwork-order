/**
 * Boot theme resolver for the SDKWork Order PC surface (§3.3 anti-flash).
 *
 * Per `THEME_DARKMODE_SPEC.md` §3.2 this module is the surface's theme owner:
 * it resolves the persisted preference first and falls back to the OS
 * color-scheme exactly once at boot. `main.tsx` consumes the resolved mode;
 * feature components never read `prefers-color-scheme` themselves.
 */

export type OrderThemeMode = 'light' | 'dark';

const THEME_STORAGE_KEY = 'sdkwork-order-theme';

/** Resolves the boot color mode: persisted choice, else OS preference. */
export function resolveInitialThemeMode(): OrderThemeMode {
  if (typeof window === 'undefined') {
    return 'light';
  }
  const stored = window.localStorage.getItem(THEME_STORAGE_KEY);
  if (stored === 'dark' || stored === 'light') {
    return stored;
  }
  const prefersDark = window.matchMedia?.('(prefers-color-scheme: dark)').matches ?? false;
  return prefersDark ? 'dark' : 'light';
}
