import { chromium, expect } from "@playwright/test";
import { copyFile, mkdir, readdir, stat, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { spawn, spawnSync } from "node:child_process";

import {
  environmentWithoutInheritedEngine,
  reserveLoopbackPort,
  sha256,
  waitForFile,
  waitForWebViewTarget,
} from "./lib/native-windows.mjs";

const repositoryRoot = resolve(import.meta.dirname, "..");
const defaultApp = resolve(repositoryRoot, "src-tauri", "target", "release", "morflo.exe");
const defaultFixture = resolve(repositoryRoot, "fixtures", "generated", "transparent-grid.png");
const appPath = resolve(process.env.MORFLO_NATIVE_APP ?? defaultApp);
const fixturePath = resolve(process.env.MORFLO_NATIVE_FIXTURE ?? defaultFixture);
const evidenceRoot = resolve(repositoryRoot, "work", "morflo", "native-windows");
const runId = new Date().toISOString().replaceAll(":", "-").replaceAll(".", "-");
const runRoot = join(evidenceRoot, runId);
const sourceName = "Native launch 京都 🧳 O'Reilly.png";
const sourcePath = join(runRoot, sourceName);
const existingOutput = join(runRoot, "Native launch 京都 🧳 O'Reilly.jpg");
const convertedOutput = join(runRoot, "Native launch 京都 🧳 O'Reilly (2).jpg");

if (process.platform !== "win32") {
  throw new Error("The packaged-native Morflo smoke test is Windows-only.");
}

for (const required of [appPath, fixturePath]) {
  const metadata = await stat(required).catch(() => undefined);
  if (!metadata?.isFile()) throw new Error(`Required file is unavailable: ${required}`);
}

await mkdir(runRoot, { recursive: true });
await copyFile(fixturePath, sourcePath);
await writeFile(existingOutput, "Morflo collision sentinel\n", "utf8");
const sourceHashBefore = await sha256(sourcePath);
const collisionHashBefore = await sha256(existingOutput);
const startedAt = performance.now();

let app;
let browser;
try {
  const port = await reserveLoopbackPort();
  const debugArgument = `--remote-debugging-port=${port}`;
  const existingArguments = process.env.WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS?.trim();
  const { environment: appEnvironment, removedEngineDirectoryCount } =
    await environmentWithoutInheritedEngine();
  app = spawn(appPath, [sourcePath], {
    env: {
      ...appEnvironment,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: existingArguments
        ? `${existingArguments} ${debugArgument}`
        : debugArgument,
    },
    shell: false,
    stdio: "ignore",
    windowsHide: true,
  });

  const endpoint = `http://127.0.0.1:${port}`;
  const target = await waitForWebViewTarget(endpoint, app);
  browser = await chromium.connectOverCDP(endpoint);
  const page = browser
    .contexts()
    .flatMap((context) => context.pages())
    .find((candidate) => candidate.url().includes("tauri.localhost"));
  if (!page) throw new Error("Connected to WebView2, but Morflo's app target was not present.");

  await page.getByRole("heading", { name: "Conversion queue" }).waitFor({ timeout: 30_000 });
  await page.locator(".file-identity strong", { hasText: sourceName }).waitFor();
  await page.getByText("JPEG has no transparency", { exact: true }).waitFor();
  await page.getByRole("button", { name: "Convert", exact: true }).waitFor();
  await page.screenshot({ path: join(runRoot, "01-native-ready.png") });

  await page.getByRole("button", { name: "Open engine diagnostics" }).click();
  await page.getByRole("dialog", { name: "Local media engine" }).waitFor();
  await page.getByText("Ready to convert", { exact: true }).waitFor();
  await page.waitForTimeout(350);
  const capabilitySummary = await page
    .locator(".diagnostics-section-heading > span")
    .textContent();
  await page.screenshot({ path: join(runRoot, "02-native-diagnostics.png") });
  await page.getByRole("button", { name: "Close engine diagnostics" }).click();

  await page.getByRole("button", { name: "Keep transparency", exact: true }).click();
  await expect(page.getByRole("button", { name: "PNG. Keeps transparency" })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  await expect(page.getByLabel("Resize mode")).toHaveValue("original");
  await expect(page.getByText("JPEG has no transparency", { exact: true })).toHaveCount(0);
  await page.getByRole("button", { name: "Easy to share", exact: true }).click();
  await expect(page.getByRole("button", { name: "JPEG. Easy to share" })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  await expect(page.getByLabel("Resize mode")).toHaveValue("original");
  await expect(page.getByText("JPEG has no transparency", { exact: true })).toBeVisible();
  await page.screenshot({ path: join(runRoot, "02-native-image-outcomes.png") });

  await page.getByRole("button", { name: "Convert", exact: true }).click();
  const completed = page.getByRole("button", { name: `Reveal output for ${sourceName}` });
  const failed = page.locator(".error-panel");
  const outcome = await Promise.race([
    completed.waitFor({ timeout: 120_000 }).then(() => "completed"),
    failed.waitFor({ timeout: 120_000 }).then(() => "failed"),
  ]);
  if (outcome === "failed") {
    await page.screenshot({ path: join(runRoot, "03-native-failed.png") });
    const message = await failed.innerText();
    const technical = await failed.locator(".error-code code").textContent();
    throw new Error(`Native conversion failed: ${message}\n${technical ?? ""}`);
  }
  await waitForFile(convertedOutput, 15_000);
  const receiptHeading = page.getByRole("heading", { name: "Your JPEG is ready" });
  await receiptHeading.waitFor();
  const outputPreview = page.locator(".result-preview.has-preview img");
  await outputPreview.waitFor({ timeout: 30_000 });
  if ((await page.locator(".result-preview.has-alpha").count()) !== 0) {
    throw new Error("The JPEG receipt incorrectly implied retained transparency.");
  }
  await outputPreview.evaluate((image) => {
    if (typeof image.naturalWidth !== "number" || image.naturalWidth === 0) {
      throw new Error("The bounded output preview did not decode.");
    }
  });
  const receiptText = await page.locator(".completion-panel").innerText();
  await page.screenshot({ path: join(runRoot, "03-native-converted.png") });

  const sourceHashAfter = await sha256(sourcePath);
  const collisionHashAfter = await sha256(existingOutput);
  if (sourceHashBefore !== sourceHashAfter) throw new Error("The source file changed.");
  if (collisionHashBefore !== collisionHashAfter) {
    throw new Error("The pre-existing collision target changed.");
  }

  const leftovers = (await readdir(runRoot)).filter((name) => name.includes(".morflo-part"));
  if (leftovers.length > 0) {
    throw new Error(`Recognized partial outputs remain: ${leftovers.join(", ")}`);
  }

  const probe = spawnSync(
    "ffprobe",
    [
      "-v",
      "error",
      "-select_streams",
      "v:0",
      "-show_entries",
      "stream=codec_name,width,height",
      "-of",
      "json",
      convertedOutput,
    ],
    { encoding: "utf8", shell: false, windowsHide: true },
  );
  if (probe.status !== 0) {
    throw new Error(`ffprobe rejected Morflo's output: ${probe.stderr.trim()}`);
  }

  const result = {
    environment:
      "Windows 11 x64; installed/release Tauri WebView2 attached over process-local CDP",
    appPath,
    webviewTarget: { title: target.title, url: target.url },
    capabilitySummary: capabilitySummary?.trim(),
    inheritedEnginePathEntriesRemoved: removedEngineDirectoryCount,
    journey:
      "startup transparent PNG -> keep transparency -> sharing without enlargement -> explicit alpha warning -> JPEG -> measured receipt -> collision-safe suffixed output",
    sourceName,
    sourceHashPreserved: true,
    existingOutputPreserved: true,
    convertedOutput,
    outputProbe: JSON.parse(probe.stdout),
    receiptHeading: await receiptHeading.textContent(),
    receiptText,
    outputPreviewRendered: true,
    outputPreviewCorrectlyOpaque: true,
    partialOutputsRemaining: leftovers.length,
    elapsedMs: Math.round(performance.now() - startedAt),
    screenshots: [
      join(runRoot, "01-native-ready.png"),
      join(runRoot, "02-native-diagnostics.png"),
      join(runRoot, "02-native-image-outcomes.png"),
      join(runRoot, "03-native-converted.png"),
    ],
  };
  await writeFile(join(runRoot, "result.json"), `${JSON.stringify(result, null, 2)}\n`, "utf8");
  process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
} finally {
  await browser?.close().catch(() => undefined);
  if (app && app.exitCode === null) {
    app.kill();
    await Promise.race([
      new Promise((resolveExit) => app.once("exit", resolveExit)),
      new Promise((resolveTimeout) => setTimeout(resolveTimeout, 3_000)),
    ]);
  }
}
