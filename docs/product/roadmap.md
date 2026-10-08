# Morflo roadmap

Morflo's next milestones serve ordinary Windows users who want local image
conversion and compression without engine setup. Keep the short path from
file intake to a useful, verified result. Scope expands when a task has demand,
an implementation boundary and acceptance evidence. Dates are not promised.

## Current source checkpoint

0.3.0 adds image outcomes, capability-aware recommendations, user guides and
repository contribution paths. Common image processing remains built in.
Public signed installers are a separate distribution milestone.

## Next priorities

| Priority | Outcome                                 | Evidence required                                                                                                                                            |
| -------- | --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| P0       | A trustworthy first installation        | Clean Windows 11 machine with no FFmpeg completes JPEG compression, PNG conversion, alpha preservation and a batch; signing and distribution review complete |
| P0       | Understandable results                  | Voluntary first-use study with 10 target users; proposed goal: at least 8 complete the assigned task without guidance                                        |
| P1       | Better size confidence                  | Bounded real encode preview with measured bytes, cancellation, cleanup and explicit quality tradeoffs                                                        |
| P1       | Target-size images                      | Bounded attempts, clear quality floor, explicit permission for resizing and an honest unattainable result                                                    |
| P1       | More efficient web images without setup | Evaluate a lossy WebP encoder against photos, text, alpha, color, speed, memory, package size and redistribution requirements                                |
| P1       | Phone-photo conversion                  | A reviewed HEIC/HEIF decoder and synthetic fixtures proving orientation, color, corruption handling and common-device interoperability                       |
| P2       | Broader platforms and video             | Real packaged platform journeys and a separately reviewed engine build before claiming availability                                                          |

Outcomes retain the current metadata and animation choices. Future convenience
must preserve that explicit consent, source immutability and safe output naming.
AVIF, custom reusable presets and localization follow evidence of demand and
maintenance capacity. English remains the product locale under the current
contract. A localization expansion needs its own scope and QA decision.

## Discovery and useful documentation

The first discoverability work is a clear README, an accurate support matrix,
an app screenshot and practical guides for JPEG compression, PNG to JPEG,
WebP to PNG and batch resizing. Each guide must reflect shipped controls and
explain its limits. No unsupported-format landing pages or generic format-count
claims.

A later static product site can reuse these guides with distinct titles,
canonical URLs, crawlable HTML, a sitemap, accessible screenshots and direct
links to reviewed releases. Publish only after choosing the destination and
reviewing distribution and hosting costs. Avoid a server-side conversion
service, uploaded media, accounts and application telemetry.

For each meaningful release, prepare one short demonstration, one practical
guide and an accurate changelog. Share in relevant communities according to
their current rules; do not bulk-post or manufacture stars, reviews or backlinks.
Consider WinGet only after an eligible public installer exists.

## Learn without application analytics

Use voluntary usability sessions and issue themes for task success. Review
GitHub visitors and release asset downloads, distinguishing downloads from
actual users. GitHub traffic has a rolling 14-day window; save dated aggregate
summaries if longer comparisons are needed. Use Search Console for a future
verified site. These are separate observations, not a joined user-tracking funnel.

## Research behind the priorities

- [Caesium](https://saerasoft.com/caesium/) presents compression, previews and
  batch work together. Morflo's opportunity is a clear local outcome with
  source safety and truthful capability limits; this is a positioning hypothesis.
- [Converseen](https://converseen.fasterland.net/) emphasizes broad formats and
  batch operations. Additional format count alone is unlikely to distinguish
  Morflo; task success is the proposed priority.
- [Google's helpful-content guidance](https://developers.google.com/search/docs/fundamentals/creating-helpful-content)
  supports focused, original instructions that help readers finish a task.
- [GitHub topics](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/classifying-your-repository-with-topics)
  improve topic discovery; they do not establish an external search ranking.
- [GitHub traffic](https://docs.github.com/en/repositories/viewing-activity-and-data-for-your-repository/viewing-traffic-to-a-repository)
  documents the short measurement window and available aggregate observations.
- [WinGet submission](https://learn.microsoft.com/en-us/windows/package-manager/package/repository)
  requires a manifest and review against an actual installer source.

Research refreshed 2026-10-09. Priorities are proposals; the current source
checkpoint and its checks are recorded in [status](../STATUS.md).
