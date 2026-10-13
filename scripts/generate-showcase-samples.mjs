import { mkdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { chromium } from "@playwright/test";

// Original procedural illustrations, authored for Morflo. No external media.
const directory = resolve(process.argv[2] ?? "work/showcase/samples");
await mkdir(directory, { recursive: true });
const samples = [
  {
    name: "Coastal morning.png",
    width: 2400,
    height: 1600,
    body: `<defs><linearGradient id="sky" x2="0" y2="1"><stop stop-color="#ecd6b6"/><stop offset="1" stop-color="#f8ede0"/></linearGradient><linearGradient id="sea" x2="1" y2="1"><stop stop-color="#648f8a"/><stop offset="1" stop-color="#183e40"/></linearGradient></defs><path fill="url(#sky)" d="M0 0h1200v800H0z"/><circle cx="880" cy="220" r="110" fill="#dc814d"/><path fill="#85978a" d="M0 370 220 190 500 400 710 335 1200 475v325H0z"/><path fill="url(#sea)" d="M0 485q420-105 1200 5v310H0z"/><path fill="#f2debd" d="M0 510q600 30 880 290H0z"/><path fill="#bb7752" d="M0 600q310-80 610 200H0z"/><path fill="#334d40" d="M0 300q110 70 190 290L0 680z"/><g fill="none" stroke="#e7eee2" stroke-width="3" opacity=".55"><path d="M570 535q180-30 470-10M680 585l450-20M820 645l350-22M1010 710l190-15"/></g><path fill="#fbf2e2" d="m780 370-65 93h65z"/><path fill="#333e36" d="m705 467 113-4-25 20h-66z"/>`,
  },
  {
    name: "Citrus study.png",
    width: 1800,
    height: 2200,
    body: `<defs><radialGradient id="fruit"><stop stop-color="#f9bd43"/><stop offset="1" stop-color="#d77627"/></radialGradient><linearGradient id="wall" x2="1" y2="1"><stop stop-color="#f1e3ca"/><stop offset="1" stop-color="#cbb994"/></linearGradient></defs><path fill="url(#wall)" d="M0 0h1200v800H0z"/><path fill="#c58158" d="M0 580h1200v220H0z"/><ellipse cx="650" cy="665" rx="350" ry="66" fill="#8a533e" opacity=".35"/><ellipse cx="600" cy="633" rx="360" ry="85" fill="#faf0d5"/><ellipse cx="600" cy="620" rx="290" ry="58" fill="#dbc9a5"/><circle cx="500" cy="525" r="134" fill="url(#fruit)"/><circle cx="720" cy="560" r="113" fill="url(#fruit)"/><path fill="#345947" d="M511 389q-170-190-220-42 96 98 220 42M514 395q83-156 152-81-24 92-152 81"/><path fill="none" stroke="#678267" stroke-width="5" d="M330 353q88 55 180 40"/><path fill="#ffdc7d" d="M630 589a116 116 0 0 0 164 99 122 122 0 0 1-164-99"/><g fill="#fff3c6" opacity=".4"><ellipse cx="453" cy="475" rx="34" ry="19" transform="rotate(-38 453 475)"/><ellipse cx="686" cy="515" rx="24" ry="13" transform="rotate(-35 686 515)"/></g>`,
  },
  {
    name: "Botanical cutout.png",
    width: 1200,
    height: 1200,
    body: `<path fill="none" stroke="#365343" stroke-width="13" stroke-linecap="round" d="M600 730Q670 490 563 117M613 592 367 355M618 492 837 271"/><g fill="#4d7960"><path d="M587 318q-253-21-222-186 204-3 222 186M613 448q-261 15-278-139 199-47 278 139M607 589q-244 74-302-52 158-107 302 52"/><path d="M601 343q205-71 208-226-216 18-208 226M621 497q283-20 282-166-211-25-282 166M628 623q258 36 299-104-203-75-299 104"/></g><g fill="#95ae79"><path d="M600 362q-166-123-143-280 119 13 143 280M626 506q166-94 168-185-145 1-168 185M626 639q185-36 200-111-143-24-200 111"/></g>`,
  },
];
const browser = await chromium.launch();
try {
  const page = await browser.newPage();
  for (const sample of samples) {
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${sample.width}" height="${sample.height}" viewBox="0 0 1200 800" preserveAspectRatio="xMidYMid slice">${sample.body}</svg>`;
    await page.setViewportSize({ width: sample.width, height: sample.height });
    await page.setContent(`<style>body{margin:0}svg{display:block}</style>${svg}`);
    await page.screenshot({ path: resolve(directory, sample.name), omitBackground: true });
  }
} finally {
  await browser.close();
}
await writeFile(
  resolve(directory, "provenance.json"),
  JSON.stringify(
    {
      creator: "Snowy",
      license: "MIT OR Apache-2.0",
      source: "scripts/generate-showcase-samples.mjs",
      method:
        "Original procedural SVG illustrations rasterized locally in Chromium; no external media",
      samples: samples.map(({ name, width, height }) => ({ name, width, height })),
    },
    null,
    2,
  ),
);
console.log(directory);
