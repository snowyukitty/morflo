/* global document, window */
import { chromium, expect } from "@playwright/test";
import { copyFile, mkdir, readdir, readFile, stat, writeFile } from "node:fs/promises";
import { resolve, join } from "node:path";
import { spawn, spawnSync } from "node:child_process";
import { reserveLoopbackPort, waitForWebViewTarget, sha256 } from "./lib/native-windows.mjs";

const label = process.argv[2] ?? "after";
const appPath = resolve(process.env.MORFLO_NATIVE_APP ?? "src-tauri/target/release/morflo.exe");
const root = resolve("work/showcase", label);
const canonicalSamples = resolve("work/showcase/samples");
const sampleRoot = join(root, `media-${Date.now()}`);
await mkdir(sampleRoot, { recursive: true });
for (const name of [
  "Coastal morning.png",
  "Citrus study.png",
  "Botanical cutout.png",
  "provenance.json",
])
  await copyFile(join(canonicalSamples, name), join(sampleRoot, name));
await mkdir(root, { recursive: true });
const sources = ["Coastal morning.png", "Citrus study.png", "Botanical cutout.png"].map(
  (name) => join(sampleRoot, name),
);
const hashes = await Promise.all(sources.map(sha256));
const port = await reserveLoopbackPort();
const app = spawn(appPath, sources, {
  shell: false,
  windowsHide: true,
  stdio: "ignore",
  env: {
    ...process.env,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
  },
});
let browser;
try {
  await waitForWebViewTarget(`http://127.0.0.1:${port}`, app);
  browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
  const findPage = () =>
    browser
      .contexts()
      .flatMap((context) => context.pages())
      .find((p) => p.url().includes("tauri.localhost"));
  await expect.poll(() => Boolean(findPage()), { timeout: 15000 }).toBe(true);
  const page = findPage();
  await expect(page.locator(".file-row")).toHaveCount(3);
  await page.evaluate(() => {
    document.documentElement.dataset.theme = "light";
  });
  const capture = async (name, width = 1280, height = 800) => {
    const path = join(root, `${name}-window.png`);
    const scale = await page.evaluate(() => window.devicePixelRatio);
    const result = spawnSync(
      "powershell",
      [
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        resolve("scripts/capture-native-window.ps1"),
        "-ProcessId",
        String(app.pid),
        "-Output",
        path,
        "-Width",
        String(Math.round(width * scale)),
        "-Height",
        String(Math.round(height * scale)),
      ],
      { windowsHide: true, encoding: "utf8" },
    );
    if (result.status !== 0) throw new Error(result.stderr);
    await page.screenshot({ path: join(root, `${name}.png`) });
  };
  await page.getByRole("button", { name: "Select all", exact: true }).click();
  await page.getByRole("button", { name: "Easy to share", exact: true }).click();
  // Keep the alpha-bearing source explicit instead of flattening it for a nicer claim.
  await page.locator(".file-row-main").filter({ hasText: "Botanical cutout.png" }).click();
  await page.getByRole("button", { name: "Keep transparency", exact: true }).click();
  await page.locator(".file-row-main").filter({ hasText: "Coastal morning.png" }).click();
  await page.waitForTimeout(1800);
  await capture("queued");
  const queuedHeights = await page
    .locator(".file-row")
    .evaluateAll((rows) => rows.map((row) => row.getBoundingClientRect().height));
  await page.getByRole("button", { name: "Convert 3 files", exact: true }).click();
  await expect(page.locator(".file-row.status-succeeded")).toHaveCount(3, { timeout: 120_000 });
  await page.waitForTimeout(2200);
  await capture("completed");
  if (label !== "before") {
    const completedHeights = await page
      .locator(".file-row")
      .evaluateAll((rows) => rows.map((row) => row.getBoundingClientRect().height));
    if (queuedHeights.some((height, index) => Math.abs(height - completedHeights[index]) > 1))
      throw new Error("Ledger rows moved when results arrived");
    await page.locator(".conversion-ledger").screenshot({ path: join(root, "ledger.png") });
    const crop = await page.locator(".conversion-ledger").evaluate((ledger) => {
      const start = ledger.querySelector(".ledger-total").getBoundingClientRect();
      const end = ledger.querySelector(".file-list").getBoundingClientRect();
      const bounds = ledger.getBoundingClientRect();
      return {
        x: bounds.x + 12,
        y: start.y,
        width: bounds.width - 24,
        height: end.bottom - start.y,
      };
    });
    await page.screenshot({ path: join(root, "ledger-detail.png"), clip: crop });
    await writeFile(
      join(root, "crop.json"),
      JSON.stringify(
        {
          crop,
          devicePixelRatio: await page.evaluate(() => window.devicePixelRatio),
          method:
            "Unaltered crop of actual Tauri client area, from the measured total to the three completed ledger rows.",
        },
        null,
        2,
      ),
    );
  }
  if (label !== "before") {
    // Both compositions operate on the same live Tauri session and measured outputs.
    const study = await page.addStyleTag({
      content: `.workspace-main {grid-template-columns:1fr!important;grid-template-rows:minmax(0,1fr) 225px!important}.inspector{min-height:0!important;display:grid!important;grid-template-columns:230px 1fr}.completion-panel{display:grid;grid-template-columns:170px 160px 1fr;gap:12px;padding:12px}.completion-heading,.completion-trust,.result-preview-caption{display:none}.result-preview{min-height:130px!important;margin:0}.result-flow,.result-file{margin:0}.inspector-header{border-right:1px solid var(--line)}.ledger-footnote{display:none}.ledger-total{padding:10px 0;margin-bottom:4px}.conversion-ledger .file-row{min-height:122px}.result-facts{align-self:center}`,
    });
    await capture("composition-b-workbench");
    await study.evaluate((node) => node.remove());
    await capture("composition-a-ledger");
    for (const [width, height] of [
      [960, 640],
      [780, 620],
    ])
      await capture(`completed-${width}x${height}`, width, height);
    await page.evaluate(() => {
      document.documentElement.dataset.theme = "dark";
    });
    await capture("completed-dark", 1280, 800);
  }
  const after = await Promise.all(sources.map(sha256));
  if (JSON.stringify(hashes) !== JSON.stringify(after)) throw new Error("Source bytes changed");
  const receipt = await page.locator(".file-list").innerText();
  const outputs = (await readdir(sampleRoot)).filter(
    (n) => /\.(jpg|png)$/.test(n) && !sources.some((p) => p === join(sampleRoot, n)),
  );
  const outputMeasurements = await Promise.all(
    outputs.map(async (name) => {
      const path = join(sampleRoot, name);
      const probe = spawnSync(
        "ffprobe",
        [
          "-v",
          "error",
          "-show_entries",
          "stream=codec_name,width,height,pix_fmt",
          "-of",
          "json",
          path,
        ],
        { encoding: "utf8", shell: false, windowsHide: true },
      );
      if (probe.status !== 0) throw new Error(`Output probe failed for ${name}`);
      return {
        name,
        sizeBytes: (await stat(path)).size,
        sha256: await sha256(path),
        probe: JSON.parse(probe.stdout),
      };
    }),
  );
  await writeFile(
    join(root, "evidence.json"),
    JSON.stringify(
      {
        sourceCommit: spawnSync("git", ["rev-parse", "HEAD"], {
          encoding: "utf8",
        }).stdout.trim(),
        appSha256: await sha256(appPath),
        sourceHashes: hashes,
        sourceUnchanged: true,
        sources: sources.map((p) => p.split(/[\\/]/).at(-1)),
        outputs,
        outputMeasurements,
        receipt,
        provenance: JSON.parse(await readFile(join(sampleRoot, "provenance.json"), "utf8")),
      },
      null,
      2,
    ),
  );
  console.log(`Captured ${label} with three real conversions; original hashes preserved.`);
} finally {
  await browser?.close().catch(() => undefined);
  if (app.exitCode === null) {
    app.kill();
    await Promise.race([
      new Promise((done) => app.once("exit", done)),
      new Promise((done) => setTimeout(done, 3000)),
    ]);
  }
}
