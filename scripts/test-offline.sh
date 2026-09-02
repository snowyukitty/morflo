#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ -z "${MORFLO_FFMPEG_PATH:-}" || -z "${MORFLO_FFPROBE_PATH:-}" ]]; then
  echo "Set MORFLO_FFMPEG_PATH and MORFLO_FFPROBE_PATH to a reviewed local engine pair." >&2
  exit 2
fi
if ! unshare --user --map-root-user --net true 2>/dev/null; then
  echo "This host does not permit an isolated user/network namespace." >&2
  exit 2
fi

target_directory="${CARGO_TARGET_DIR:-/tmp/morflo-offline-target}"
unshare --user --map-root-user --net env \
  CARGO_NET_OFFLINE=true \
  CARGO_TARGET_DIR="$target_directory" \
  MORFLO_FFMPEG_PATH="$MORFLO_FFMPEG_PATH" \
  MORFLO_FFPROBE_PATH="$MORFLO_FFPROBE_PATH" \
  cargo test \
    --offline \
    --manifest-path "$repository_root/src-tauri/Cargo.toml" \
    --features real-engine \
    --test real_engine \
    -- \
    --ignored \
    --nocapture
