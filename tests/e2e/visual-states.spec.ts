import { mkdir } from "node:fs/promises";
import { resolve } from "node:path";

import { expect, test, type Browser, type Page } from "@playwright/test";
import axe from "axe-core";

const screenshotRoot = resolve("screenshots", "release");

function appUrl(query: string): string {
  const baseUrl = process.env.MORFLO_E2E_BASE_URL;
  if (!baseUrl) throw new Error("MORFLO_E2E_BASE_URL is not set by global setup");
  return `${baseUrl}/?${query}`;
}

async function settle(page: Page, query: string): Promise<void> {
  await page.goto(appUrl(query));
  await page.evaluate(async () => {
    await document.fonts.ready;
  });
  const expectedTheme = new URLSearchParams(query).get("theme");
  if (expectedTheme) {
    await expect(page.locator("html")).toHaveAttribute("data-theme", expectedTheme);
  }
  await expect(page.locator("#root")).toBeVisible();
  await page.waitForTimeout(260);
}

test.beforeAll(async () => {
  await mkdir(screenshotRoot, { recursive: true });
});

const productStates = [
  { name: "empty-light-1440x900", query: "demo=empty&theme=light", width: 1440, height: 900 },
  { name: "queue-light-1440x900", query: "demo=queue&theme=light", width: 1440, height: 900 },
  { name: "active-1280x800", query: "demo=active&theme=light", width: 1280, height: 800 },
  { name: "video-to-gif-1280x800", query: "demo=gif&theme=light", width: 1280, height: 800 },
  {
    name: "video-to-gif-dark-1280x800",
    query: "demo=gif&theme=dark",
    width: 1280,
    height: 800,
  },
  {
    name: "video-to-gif-narrow-780x620",
    query: "demo=gif&theme=light",
    width: 780,
    height: 620,
    scrollInspector: true,
  },
  {
    name: "completed-1440x900",
    query: "demo=complete&theme=light",
    width: 1440,
    height: 900,
  },
  {
    name: "partial-completion-1280x800",
    query: "demo=partial&selection=all&theme=light",
    width: 1280,
    height: 800,
  },
  {
    name: "completed-dark-1280x800",
    query: "demo=complete&theme=dark",
    width: 1280,
    height: 800,
  },
  { name: "error-1280x800", query: "demo=error&theme=light", width: 1280, height: 800 },
  { name: "queue-dark-1440x900", query: "demo=queue&theme=dark", width: 1440, height: 900 },
  {
    name: "engine-diagnostics-ready-1280x800",
    query: "demo=queue&panel=diagnostics&theme=light",
    width: 1280,
    height: 800,
  },
  {
    name: "engine-diagnostics-missing-dark-1280x800",
    query: "demo=engine-missing&panel=diagnostics&theme=dark",
    width: 1280,
    height: 800,
  },
  {
    name: "engine-built-in-1280x800",
    query: "demo=engine-native&theme=light",
    width: 1280,
    height: 800,
  },
  {
    name: "engine-diagnostics-built-in-1280x800",
    query: "demo=engine-native&panel=diagnostics&theme=light",
    width: 1280,
    height: 800,
  },
  { name: "narrow-780x620", query: "demo=queue&theme=light", width: 780, height: 620 },
  {
    name: "completed-narrow-780x620",
    query: "demo=complete&theme=light",
    width: 780,
    height: 620,
    scrollInspector: true,
  },
  {
    name: "design-directions-1440x900",
    query: "study=design&theme=light",
    width: 1440,
    height: 900,
  },
] as const;

for (const state of productStates) {
  test(`captures ${state.name}`, async ({ page }) => {
    await page.setViewportSize({ width: state.width, height: state.height });
    await settle(page, state.query);
    if ("scrollInspector" in state) {
      await page.locator(".inspector").scrollIntoViewIfNeeded();
    }
    await page.screenshot({ path: resolve(screenshotRoot, `${state.name}.png`) });
  });
}

async function captureScale(browser: Browser, scale: number, name: string): Promise<void> {
  const context = await browser.newContext({
    viewport: { width: 1280, height: 800 },
    deviceScaleFactor: scale,
    colorScheme: "light",
    reducedMotion: "reduce",
  });
  try {
    const page = await context.newPage();
    await settle(page, "demo=queue&theme=light");
    await page.screenshot({ path: resolve(screenshotRoot, name) });
  } finally {
    await context.close();
  }
}

test("captures scaling evidence", async ({ browser }) => {
  await captureScale(browser, 1, "queue-scale-100.png");
  await captureScale(browser, 1.25, "queue-scale-125.png");
  await captureScale(browser, 1.5, "queue-scale-150.png");
});

test("has no serious browser accessibility violations", async ({ page }) => {
  for (const query of [
    "demo=empty&theme=light",
    "demo=queue&theme=light",
    "demo=gif&theme=dark",
    "demo=error&theme=light",
    "demo=complete&theme=dark",
    "demo=engine-missing&panel=diagnostics&theme=dark",
  ]) {
    await page.setViewportSize({ width: 1280, height: 800 });
    await settle(page, query);
    await page.addScriptTag({ content: axe.source });
    const violations = await page.evaluate(async () => {
      const axeApi = (
        window as typeof window & {
          axe: { run: () => Promise<{ violations: { impact: string | null; id: string }[] }> };
        }
      ).axe;
      const result = await axeApi.run();
      return result.violations.filter(
        (violation) => violation.impact === "critical" || violation.impact === "serious",
      );
    });
    expect(violations, query).toEqual([]);
  }
});

test("presents measured individual and batch outcomes", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await settle(page, "demo=complete&theme=light");
  await expect(page.getByRole("heading", { name: "Your MP4 is ready" })).toBeVisible();
  await expect(page.getByText("61% smaller")).toBeVisible();

  await settle(page, "demo=partial&theme=light");
  await page.getByRole("button", { name: "Select all" }).click();
  await expect(page.getByRole("heading", { name: "1 of 2 outputs is ready" })).toBeVisible();
  await expect(page.getByText("1 needs attention")).toBeVisible();
});

test("offers image outcomes without an external engine at desktop and minimum sizes", async ({
  page,
}) => {
  for (const theme of ["light", "dark"]) {
    for (const width of [1280, 780]) {
      await page.setViewportSize({ width, height: width === 780 ? 620 : 800 });
      await settle(page, `demo=engine-native&theme=${theme}`);
      await page.getByRole("button", { name: "Smaller file", exact: true }).click();
      await page.locator(".manual-settings > summary").click();
      await expect(
        page.getByRole("button", { name: "PNG. Keeps transparency" }),
      ).toHaveAttribute("aria-pressed", "true");
      await expect(page.getByText("PNG · Keeps transparency · Size may grow")).toBeVisible();
      await page.getByText("O'Reilly cup.jpg", { exact: true }).click();
      await expect(page.getByRole("button", { name: "JPEG. Easy to share" })).toHaveAttribute(
        "aria-pressed",
        "true",
      );
      await expect(
        page.getByRole("button", { name: "WebP. Smaller web image" }),
      ).toBeDisabled();
      const share = page.getByRole("button", { name: "Easy to share", exact: true });
      await share.focus();
      await page.keyboard.press("Enter");
      await expect(share).toHaveAttribute("aria-pressed", "true");
      await expect(page.getByLabel("Resize mode")).toHaveValue("contain");
      await expect(page.getByLabel(/^Width/)).toHaveValue("1920");
      await share.scrollIntoViewIfNeeded();
      await page.screenshot({
        path: resolve(screenshotRoot, `image-sharing-${theme}-${width}.png`),
      });
    }
  }
});

test("keeps the GIF range direct, constrained, and keyboard ordered", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await settle(page, "demo=gif&theme=light");
  const start = page.getByRole("slider", { name: "GIF start time" });
  const end = page.getByRole("slider", { name: "GIF end time" });
  await expect(start).toHaveAttribute("max", "5.5");
  await expect(end).toHaveAttribute("min", "0.5");

  const strip = page.locator(".moment-strip");
  const bounds = await strip.boundingBox();
  if (!bounds) throw new Error("The GIF moment strip was not measurable.");
  await strip.click({ position: { x: bounds.width * 0.75, y: bounds.height / 2 } });
  await expect(end).toHaveValue("9.3");
  await expect(page.getByText("9.3 sec selected")).toBeVisible();

  await start.focus();
  await page.keyboard.press("Tab");
  await expect(end).toBeFocused();
});

test("keeps the completed-output action reachable at the minimum window size", async ({
  page,
}) => {
  await page.setViewportSize({ width: 780, height: 620 });
  await settle(page, "demo=complete&theme=light");
  const reveal = page.getByRole("button", { name: "Reveal in folder" });
  await reveal.scrollIntoViewIfNeeded();
  await expect(reveal).toBeVisible();
  await expect(reveal).toBeEnabled();
});

test("remains operable at 200 percent text zoom", async ({ page }) => {
  await page.setViewportSize({ width: 780, height: 620 });
  await settle(page, "demo=queue&theme=light");
  await page.evaluate(() => {
    document.documentElement.style.fontSize = "200%";
  });
  await expect(page.getByRole("button", { name: "Convert 3 files" })).toBeVisible();
  await expect(page.getByRole("button", { name: /add files/i })).toBeVisible();
});

test("ledger keeps facts and recovery actions reachable across window sizes", async ({
  page,
}) => {
  for (const width of [1280, 960, 780]) {
    for (const theme of ["light", "dark"]) {
      await page.setViewportSize({
        width,
        height: width === 1280 ? 800 : width === 960 ? 640 : 620,
      });
      await settle(page, `demo=partial&theme=${theme}`);
      await expect(page.locator(".ledger-total")).toContainText("1 of 2 files measured");
      await expect(page.getByRole("button", { name: /^Reveal output for/ })).toBeVisible();
      const retry = page.getByRole("button", { name: /^Retry / });
      await retry.scrollIntoViewIfNeeded();
      await retry.focus();
      await page.keyboard.press("Enter");
      await expect(page.locator(".file-row.status-ready")).toHaveCount(1);
      expect(
        await page
          .locator(".file-list")
          .evaluate((node) => node.scrollWidth <= node.clientWidth + 1),
      ).toBe(true);
      await page.getByRole("button", { name: "Hide file settings" }).click();
      await expect(page.locator(".inspector")).toHaveCount(0);
      await page.getByRole("button", { name: "Show file settings" }).click();
      await expect(page.locator(".inspector")).toHaveCount(1);
    }
  }
});

test("unavailable video offers setup and preserves a recoverable source", async ({ page }) => {
  await page.setViewportSize({ width: 960, height: 640 });
  await settle(page, "demo=video-unavailable&theme=light");
  await expect(
    page.getByRole("heading", { name: "Video needs a local media engine" }),
  ).toBeVisible();
  await expect(page.locator(".file-row")).toContainText("No output created");
  await page.getByRole("button", { name: "Set up video conversion" }).click();
  await expect(page.getByRole("dialog", { name: "Conversion engine" })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("button", { name: "Set up video conversion" })).toBeFocused();
});

test("twenty-file ledger retains keyboard selection and all queue actions", async ({
  page,
}) => {
  await page.setViewportSize({ width: 960, height: 640 });
  await settle(page, "demo=batch&theme=dark");
  await expect(page.locator(".file-row")).toHaveCount(20);
  const last = page.locator(".file-row").last().getByRole("checkbox");
  await last.focus();
  await page.keyboard.press("Space");
  await expect(last).toBeChecked();
  await expect(page.getByRole("button", { name: "Convert 20 files" })).toBeVisible();
  await page.screenshot({ path: resolve(screenshotRoot, "ledger-20-dark-960.png") });
});

test("filtered batch recovery keeps successful receipts and prepares only unfinished files", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await settle(page, "demo=recovery&theme=light&selection=all");
  const total = (await page.locator(".ledger-total").textContent()) ?? "";
  const attention = page.getByRole("button", { name: "Needs attention 2", exact: true });
  await attention.focus();
  await page.keyboard.press("Enter");
  await expect(attention).toHaveAttribute("aria-pressed", "true");
  await expect(attention).toBeFocused();
  await expect(page.locator(".file-row")).toHaveCount(2);
  await expect(page.locator(".queue-summary")).toContainText("2 selected for settings");
  await expect(page.locator(".ledger-total")).toHaveText(total);
  await page.getByRole("button", { name: "Prepare 2 retries", exact: true }).click();
  await expect(page.getByRole("button", { name: "All files 4", exact: true })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  await expect(page.locator(".file-row.status-ready")).toHaveCount(3);
  await expect(page.locator(".file-row.status-succeeded")).toHaveCount(1);
  await expect(page.locator(".file-row.is-selected")).toHaveCount(2);
  await expect(page.locator(".file-row.is-selected .file-row-main").first()).toBeFocused();
  await page.getByRole("button", { name: "Easy to share", exact: true }).click();
  for (const row of await page.locator(".file-row.is-selected").all()) {
    await expect(row.locator(".target-format")).toContainText("JPEG");
    await expect(row.locator(".row-state")).toContainText("Not measured");
  }
  await expect(page.locator('[data-file-id="canceled-image"]')).toContainText(
    "Fit within 1920 × 1920",
  );
  const untouched = page
    .locator(".file-row")
    .filter({ has: page.getByRole("checkbox", { name: "Select O'Reilly cup.jpg" }) });
  await expect(untouched).toContainText("4032 × 3024");
  await expect(untouched).not.toContainText("Fit within");
  await expect(page.locator(".ledger-total")).toHaveText(total);
  // Filtering never narrows the explicitly queue-wide Convert action.
  await page.getByRole("button", { name: "Completed 1", exact: true }).click();
  await expect(page.locator(".file-row")).toHaveCount(1);
  await page.getByRole("button", { name: "Convert 3 files", exact: true }).click();
  await expect(page.getByRole("button", { name: "Cancel all", exact: true })).toBeVisible();
  await expect(page.locator(".file-row.status-succeeded")).toHaveCount(1);
});

test("empty filters, keyboard removal and recovery stay usable at minimum size", async ({
  page,
}) => {
  await page.setViewportSize({ width: 780, height: 620 });
  await settle(page, "demo=recovery&theme=dark&selection=all");
  await page.getByRole("button", { name: "Needs attention 2", exact: true }).click();
  await page.keyboard.press("Delete");
  await expect(
    page.getByRole("button", { name: "Needs attention 0", exact: true }),
  ).toBeFocused();
  await expect(page.getByText("No files need attention", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Convert", exact: true })).toBeEnabled();
  await page.getByRole("button", { name: "Show all files", exact: true }).click();
  await expect(page.locator(".file-row")).toHaveCount(2);
  await expect(page.locator(".file-row.status-succeeded")).toHaveCount(1);
  await expect(page.locator(".file-row.status-ready")).toHaveCount(1);
  expect(
    await page
      .locator(".conversion-ledger")
      .evaluate((node) => node.scrollWidth <= node.clientWidth + 1),
  ).toBe(true);
  await page.screenshot({ path: resolve(screenshotRoot, "recovery-minimum-dark.png") });
});
