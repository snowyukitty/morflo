import { describe, expect, test } from "vitest";

import { defaultPreferences, loadPreferences, savePreferences } from "./preferences";

describe("versioned local preferences", () => {
  test("round-trips only path-free versioned values", () => {
    const storage = window.localStorage;
    storage.clear();
    const preferences = {
      ...defaultPreferences,
      theme: "dark" as const,
      collisionPolicy: "skip" as const,
    };

    savePreferences(preferences, storage);

    expect(loadPreferences(storage)).toEqual(preferences);
    expect(storage.getItem("morflo.preferences")).not.toContain("path");
  });

  test("migrates the legacy theme and rejects an unknown schema", () => {
    const storage = window.localStorage;
    storage.clear();
    storage.setItem("morflo.theme", "dark");
    expect(loadPreferences(storage).theme).toBe("dark");

    storage.setItem("morflo.preferences", JSON.stringify({ schemaVersion: 99, theme: "dark" }));
    storage.removeItem("morflo.theme");
    expect(loadPreferences(storage)).toEqual(defaultPreferences);
  });
});
