import { chromium, expect } from "@playwright/test";
import { copyFile, mkdir, readdir, rename, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { spawn } from "node:child_process";
import { reserveLoopbackPort, sha256, waitForWebViewTarget } from "./lib/native-windows.mjs";

const runRoot = resolve(
  "work/morflo/native-ledger",
  new Date().toISOString().replaceAll(":", "-"),
);
await mkdir(runRoot, { recursive: true });
const sampleNames = ["Coastal morning.png", "Citrus study.png", "Botanical cutout.png"];
const sources = [];
for (let i = 0; i < 20; i++) {
  const path = join(runRoot, `${String(i + 1).padStart(2, "0")}-${sampleNames[i % 3]}`);
  await copyFile(resolve("work/showcase/samples", sampleNames[i % 3]), path);
  sources.push(path);
}
const originalHashes = await Promise.all(sources.map(sha256));
const damagedSource = join(runRoot, "Uninspectable.png");
await writeFile(damagedSource, "Deliberately invalid image data for recovery testing.");
const damagedHash = await sha256(damagedSource);
const port = await reserveLoopbackPort();
const app = spawn(
  resolve(process.env.MORFLO_NATIVE_APP ?? "src-tauri/target/release/morflo.exe"),
  [...sources, damagedSource],
  {
    shell: false,
    windowsHide: true,
    stdio: "ignore",
    env: {
      ...process.env,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
    },
  },
);
let browser;
try {
  await waitForWebViewTarget(`http://127.0.0.1:${port}`, app);
  browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
  const findPage = () =>
    browser
      .contexts()
      .flatMap((c) => c.pages())
      .find((p) => p.url().includes("tauri.localhost"));
  await expect.poll(() => Boolean(findPage()), { timeout: 15000 }).toBe(true);
  const page = findPage();
  await expect(page.locator(".file-row")).toHaveCount(21);
  await expect(page.locator(".file-row.status-failed")).toHaveCount(1);
  await page.getByRole("button", { name: "Select all", exact: true }).click();
  await page.getByRole("button", { name: "Easy to share", exact: true }).click();
  await page.screenshot({ path: join(runRoot, "20-queued.png") });
  // Simulate a removable/moved file after inspection. Restore it for recovery.
  await rename(sources[1], `${sources[1]}.held`);
  await rename(sources[4], `${sources[4]}.held`);
  await page.getByRole("button", { name: "Convert 20 files", exact: true }).click();
  await expect(page.locator(".file-row.status-succeeded")).toHaveCount(18, { timeout: 120000 });
  await expect(page.locator(".file-row.status-failed")).toHaveCount(3);
  await expect(page.locator(".ledger-total")).toContainText("18 of 21 files measured");
  const existingOutputs = (await readdir(runRoot)).filter((name) => name.endsWith(".jpg"));
  if (existingOutputs.length !== 18) throw new Error("Expected exactly 18 successful outputs");
  const outputHashes = await Promise.all(
    existingOutputs.map((name) => sha256(join(runRoot, name))),
  );
  await page.getByRole("button", { name: "Needs attention 3", exact: true }).click();
  await page.getByRole("button", { name: "Select shown", exact: true }).click();
  await expect(page.locator(".file-row")).toHaveCount(3);
  await expect(page.locator(".ledger-total")).toContainText("18 of 21 files measured");
  await page.screenshot({ path: join(runRoot, "18-success-3-needing-attention.png") });
  await rename(`${sources[1]}.held`, sources[1]);
  await rename(`${sources[4]}.held`, sources[4]);
  await page.getByRole("button", { name: "Prepare 3 retries", exact: true }).click();
  await expect(page.locator(".file-row.status-ready")).toHaveCount(2);
  await expect(page.locator(".file-row.is-selected")).toHaveCount(2);
  await expect(page.locator(".file-row.is-selected .file-row-main").first()).toBeFocused();
  await expect(page.locator(".file-row.status-succeeded")).toHaveCount(18);
  await expect(page.locator(".file-row.status-failed")).toHaveCount(1);
  for (const row of await page.locator(".file-row.is-selected").all())
    await expect(row).toContainText("Fit within 1920 × 1920");
  await page.getByRole("button", { name: "Convert 2 files", exact: true }).click();
  await expect(page.locator(".file-row.status-succeeded")).toHaveCount(20, { timeout: 120000 });
  await expect(page.locator(".ledger-total")).toContainText("20 of 21 files measured");
  if ((await sha256(damagedSource)) !== damagedHash) throw new Error("Damaged source changed");
  if (JSON.stringify(originalHashes) !== JSON.stringify(await Promise.all(sources.map(sha256))))
    throw new Error("Originals changed");
  const partials = (await readdir(runRoot)).filter((name) => name.includes(".morflo-part"));
  if (partials.length) throw new Error("Partial outputs remained");
  if ((await readdir(runRoot)).filter((name) => name.endsWith(".jpg")).length !== 20)
    throw new Error("Recovery repeated a successful conversion");
  if (
    JSON.stringify(outputHashes) !==
    JSON.stringify(
      await Promise.all(existingOutputs.map((name) => sha256(join(runRoot, name)))),
    )
  )
    throw new Error("Recovery changed an already completed output");
  await page.getByRole("button", { name: "Completed 20", exact: true }).click();
  await expect(page.locator(".file-row")).toHaveCount(20);
  await page.screenshot({ path: join(runRoot, "20-recovered.png") });
  await writeFile(
    join(runRoot, "result.json"),
    JSON.stringify(
      {
        batch: 20,
        properties: "Three different dimensions; opaque and transparent originals",
        partialFailure: 2,
        recovered: 2,
        existingOutputsUnchanged: true,
        successfulFilesReprocessed: false,
        settingsPreserved: true,
        failedReinspectionPreserved: true,
        damagedSourceUnchanged: true,
        success: 20,
        originalHashes,
        originalsUnchanged: true,
        partialOutputs: 0,
        receipt: await page.locator(".ledger-total").innerText(),
      },
      null,
      2,
    ),
  );
  await page.getByRole("button", { name: "Clear queue", exact: true }).click();
  const animated = join(runRoot, "Animation choice.png");
  await copyFile(resolve("fixtures/generated/animated-input.png"), animated);
  const animatedHash = await sha256(animated);
  const intake = spawn(
    resolve(process.env.MORFLO_NATIVE_APP ?? "src-tauri/target/release/morflo.exe"),
    [animated],
    { shell: false, windowsHide: true, stdio: "ignore" },
  );
  await new Promise((resolveExit, reject) => {
    intake.on("error", reject);
    intake.on("exit", resolveExit);
  });
  await expect(page.getByText("This image is animated", { exact: true })).toBeVisible();
  await expect(page.getByRole("checkbox", { name: "Use the first frame" })).not.toBeChecked();
  await page.getByRole("button", { name: "Convert", exact: true }).click();
  await expect(page.locator(".file-row.status-failed")).toHaveCount(1);
  await page
    .locator(".file-row")
    .getByRole("button", { name: /^Retry / })
    .click();
  await page.getByRole("checkbox", { name: "Use the first frame" }).check();
  await page.getByRole("button", { name: "Convert", exact: true }).click();
  await expect(page.locator(".file-row.status-succeeded")).toHaveCount(1, { timeout: 60000 });
  if ((await sha256(animated)) !== animatedHash) throw new Error("Animated original changed");
  await page.screenshot({ path: join(runRoot, "animation-explicit-first-frame.png") });
  await writeFile(
    join(runRoot, "animation.json"),
    JSON.stringify(
      {
        originalHash: animatedHash,
        implicitFlatteningRejected: true,
        explicitFirstFrameSucceeded: true,
        originalUnchanged: true,
      },
      null,
      2,
    ),
  );
  console.log(
    `Native ledger: 20 conversions, partial failure and recovery passed. Evidence: ${runRoot}`,
  );
} finally {
  await browser?.close();
  app.kill();
}
