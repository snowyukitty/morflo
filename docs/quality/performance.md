# Performance observations

These are measurements, not marketing targets. Re-run `pnpm measure:performance` on the target release hardware; ignored JSON evidence is written under `work/morflo/`.

## Environment

- Windows 11 Pro x64, build 26200
- AMD Ryzen 9 9950X, 16 cores / 32 logical processors
- 64 GB RAM
- FFmpeg/ffprobe 8.1.2 full GPL development build
- Playwright Chromium 140 through Playwright 1.62.1

## 2026-08-30 observations

| Observation                               |         Result | Boundary                                                                                 |
| ----------------------------------------- | -------------: | ---------------------------------------------------------------------------------------- |
| FFprobe cold inspection, 3 s 1080p MP4    |        88.1 ms | Direct local ffprobe child process                                                       |
| FFprobe warm mean / max, five runs        | 60.1 / 62.3 ms | Same fixture and engine                                                                  |
| First useful FFmpeg progress record       |       145.4 ms | MP4 encode with `-progress pipe:1`; not wall-clock estimation                            |
| 4096×3072 PNG → WebP                      |       524.9 ms | Development engine, balanced product-equivalent arguments                                |
| Peak FFmpeg working set for that image    |      133.5 MiB | Sampled child-process working set every 5 ms                                             |
| Seven-frame moment sampler, isolated      |         394 ms | Two-at-a-time input seeking; seven distinct 160×90 JPEG results                          |
| Seven-frame sampler in latest real suite  |         456 ms | Seven distinct bounded frames; still within the 20-second deadline                       |
| Source integrity after measurement        |      Preserved | SHA-256 before/after                                                                     |
| Frontend first useful queue render, cold  |      1314.4 ms | Vite transform server plus Chromium navigation; deliberately not called packaged startup |
| Frontend first useful queue render, warm  |       160.2 ms | Same browser context and 20-item production React tree                                   |
| 20-item selection mean / max              | 60.1 / 74.6 ms | User-visible click-to-checked assertion across 19 items                                  |
| Process-tree cancellation request         | <1 ms observed | WSL2 Rust coordinator controlling Windows FFmpeg; no process remained                    |
| Native first post-install shortcut launch |       1,843 ms | Responding Windows window after install/scan; not a full painted-content metric          |
| Native warm window-ready mean / max       |  95.7 / 142 ms | Three launches; nonzero window handle and responding process; OS caches not flushed      |
| Native main-process working set at ready  |       20.1 MiB | WebView2 child processes are separate and not included                                   |
| Installed Open with queue arrival         |       1,357 ms | Second real process with PNG + MOV → inspected rows in existing release WebView2         |
| Installed GIF moment strip ready          |       2,375 ms | Click GIF → seven decoded frames in release WebView2; includes concurrent poster work    |
| Installed MP4 → GIF receipt journey       |       6,788 ms | Startup through 2.0 s / 24-frame GIF, decoded clip and output previews                   |
| Installed process-tree cancellation       |         328 ms | UI click → Canceled; no output, partial, or direct FFmpeg child remained                 |
| Real coordinator cancellation             |         561 ms | Direct real-engine test; no process-tree survivor                                        |
| Installed PNG → JPEG receipt journey      |       5,370 ms | Startup, inspection, UI conversion, probe/finalize, decoded preview, and receipt         |

The first native measurement includes post-install and security-scanning effects,
so it is reported separately from the three warm launches. Window readiness is
repeatable process evidence, not a claim that every pixel was painted or that a
first-use conversion engine was already available.

The installed journeys intentionally remove the inherited FFmpeg `PATH` entry
and use international filenames. The 5,370 ms image observation preserves an
existing collision target and waits for its decoded receipt. The 6,788 ms GIF
observation includes seven real moments, a decoded local clip, two-stage palette
work, post-probe finalization, and a decoded animated receipt. The 1,357 ms Open
with observation begins immediately before launching the second process and
ends only after both files are visible in the first window; it is not a cold
startup measurement. None is a general engine benchmark.
