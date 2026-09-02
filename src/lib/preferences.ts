import type { CollisionPolicy, MetadataPolicy } from "../types";

export type ThemePreference = "system" | "light" | "dark";

export interface PreferencesV1 {
  schemaVersion: 1;
  theme: ThemePreference;
  collisionPolicy: CollisionPolicy;
  imageMetadata: MetadataPolicy;
  videoMetadata: MetadataPolicy;
}

export const defaultPreferences: PreferencesV1 = {
  schemaVersion: 1,
  theme: "system",
  collisionPolicy: "suffix",
  imageMetadata: "remove",
  videoMetadata: "preserve",
};

const STORAGE_KEY = "morflo.preferences";
const LEGACY_THEME_KEY = "morflo.theme";

export function loadPreferences(storage: Storage = window.localStorage): PreferencesV1 {
  try {
    const raw = storage.getItem(STORAGE_KEY);
    if (raw) {
      const value: unknown = JSON.parse(raw);
      if (isPreferencesV1(value)) return value;
    }
    const legacyTheme = storage.getItem(LEGACY_THEME_KEY);
    if (legacyTheme === "light" || legacyTheme === "dark") {
      return { ...defaultPreferences, theme: legacyTheme };
    }
  } catch {
    // Invalid or unavailable local storage falls back to path-free defaults.
  }
  return { ...defaultPreferences };
}

export function savePreferences(
  preferences: PreferencesV1,
  storage: Storage = window.localStorage,
): void {
  try {
    storage.setItem(STORAGE_KEY, JSON.stringify(preferences));
    storage.removeItem(LEGACY_THEME_KEY);
  } catch {
    // Conversion remains fully usable when private browsing disables storage.
  }
}

function isPreferencesV1(value: unknown): value is PreferencesV1 {
  if (typeof value !== "object" || value === null) return false;
  const candidate = value as Partial<PreferencesV1>;
  return (
    candidate.schemaVersion === 1 &&
    ["system", "light", "dark"].includes(candidate.theme ?? "") &&
    ["suffix", "skip"].includes(candidate.collisionPolicy ?? "") &&
    ["remove", "preserve"].includes(candidate.imageMetadata ?? "") &&
    ["remove", "preserve"].includes(candidate.videoMetadata ?? "")
  );
}
