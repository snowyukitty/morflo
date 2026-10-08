# Source publication and distribution readiness

Morflo's source repository is public and the application is licensed under
MIT OR Apache-2.0. The 0.3.0 source checkpoint improves image outcomes,
capability-aware recommendations, user documentation and contribution paths.
This is separate from releasing a signed installer.

## Current publication contract

- Publish only reviewed source and documentation, without media-engine binaries,
  local evidence directories, secrets, personal paths or account operational details.
- Preserve the public repository's existing history. Do not import unrelated
  development history or force-push a replacement.
- Inspect the exact diff and current workflow triggers before each push.
- Keep app capability and platform claims aligned with actual evidence.
- Repository publication does not authorize a release tag, binary distribution,
  visibility change, paid service or automatic deployment.

## Source checks

Run the repository quality contract and relevant conversion/native tests for
the checkpoint. Inspect source and documents for secret patterns, personal
paths and unintended external actions. Review screenshot content before
committing it. The current results belong in [status](../STATUS.md); historical
checks are not proof that a later tree passed.

The initial source review on 2026-09-03 found no credentials or email addresses
in the then-current tracked files. It removed personal paths and operational
references, added the selected licenses and verified the then-current privacy
and dependency checks. Those observations are historical.

## Distribution still requires its own evidence

Common image conversion can run with the built-in redistributable image engine.
A missing FFmpeg bundle therefore does not block the image product. Video,
animated GIF, WebP and AVIF output still need a compatible local engine unless
an exact redistributable bundle passes separate review.

The normal Windows package remains engine-free and unsigned. Trusted signing,
clean-machine first-use validation and explicit owner release authorization
remain distribution gates. macOS has no completed runtime evidence, and Linux
requires current packaged testing and assessment of its dependency graph.

See the [release checklist](release-checklist.md),
[media-engine distribution](../legal/media-engine-distribution.md), and
[roadmap](../product/roadmap.md).
