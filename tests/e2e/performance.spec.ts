import { mkdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";

import { expect, test } from "@playwright/test";

function appUrl(query: string): string {
  const baseUrl = process.env.MORFLO_E2E_BASE_URL;
  if (!baseUrl) throw new Error("MORFLO_E2E_BASE_URL is not set by global setup");
  return `${baseUrl}/?${query}`;
}

test("records bounded frontend startup and 20-file queue responsiveness", async ({ page }) => {
  const coldStarted = performance.now();
  await page.goto(appUrl("demo=batch&theme=light"));
  await expect(page.getByRole("heading", { name: "Conversion queue" })).toBeVisible();
  const coldUsefulRenderMs = performance.now() - coldStarted;

  const warmStarted = performance.now();
  await page.reload();
  await expect(page.getByRole("heading", { name: "Conversion queue" })).toBeVisible();
  const warmUsefulRenderMs = performance.now() - warmStarted;

  const selectionLatenciesMs: number[] = [];
  for (let index = 2; index <= 20; index += 1) {
    const checkbox = page.getByRole("checkbox", {
      name: `Select Product photo ${String(index).padStart(2, "0")}.jpg`,
    });
    const started = performance.now();
    await checkbox.click();
    await expect(checkbox).toBeChecked();
    selectionLatenciesMs.push(performance.now() - started);
  }

  const result = {
    environment: "Playwright Chromium with the production React tree and Vite transform server",
    coldUsefulRenderMs: Number(coldUsefulRenderMs.toFixed(1)),
    warmUsefulRenderMs: Number(warmUsefulRenderMs.toFixed(1)),
    queueItems: 20,
    selectionMeanMs: Number(
      (
        selectionLatenciesMs.reduce((sum, value) => sum + value, 0) /
        selectionLatenciesMs.length
      ).toFixed(1),
    ),
    selectionMaxMs: Number(Math.max(...selectionLatenciesMs).toFixed(1)),
  };
  const outputDirectory = resolve("work", "morflo");
  await mkdir(outputDirectory, { recursive: true });
  await writeFile(
    resolve(outputDirectory, "performance-browser.json"),
    `${JSON.stringify(result, null, 2)}\n`,
    "utf8",
  );
});
