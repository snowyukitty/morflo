#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 /absolute/path/to/Morflo.deb" >&2
  exit 2
fi

package_path="$1"
if [[ "$package_path" != /* || ! -f "$package_path" ]]; then
  echo "package path must be an existing absolute .deb file" >&2
  exit 2
fi

smoke_directory="$(mktemp -d /tmp/morflo-package-smoke.XXXXXX)"
cleanup() {
  if [[ "$smoke_directory" == /tmp/morflo-package-smoke.?????? ]]; then
    rm -rf -- "$smoke_directory"
  fi
}
trap cleanup EXIT

dpkg-deb -x "$package_path" "$smoke_directory"
file "$smoke_directory/usr/bin/morflo"
if ldd "$smoke_directory/usr/bin/morflo" | grep "not found"; then
  echo "package has unresolved native libraries" >&2
  exit 1
fi
cat "$smoke_directory/usr/share/applications/Morflo.desktop"

if strings "$smoke_directory/usr/bin/morflo" | grep -E "(/mnt/[a-z]/AI_Projects|[A-Z]:\\\\AI_Projects)"; then
  echo "package contains a private build path" >&2
  exit 1
fi

"$smoke_directory/usr/bin/morflo" >"$smoke_directory/smoke.log" 2>&1 &
app_pid=$!
sleep 4
if ! kill -0 "$app_pid" 2>/dev/null; then
  cat "$smoke_directory/smoke.log" >&2
  exit 1
fi
kill "$app_pid"
wait "$app_pid" || true
echo "package-smoke-passed"
sed -n "1,20p" "$smoke_directory/smoke.log"
