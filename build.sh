#!/bin/sh
# SPDX-License-Identifier: MIT OR Apache-2.0
# Build the PoC to its own pkg/ (never ui/src/wasm-pkg) and record what was built.
set -eu
cd "$(dirname "$0")"
wasm-pack build --target web --release --out-dir pkg --no-typescript
# wasm-pack copies a README and a catch-all .gitignore into pkg/; the first would contradict the main README, the second hides the build.
rm -f pkg/README.md pkg/.gitignore
COMMIT="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
printf '{"commit":"%s","crates":{"orchard":"0.15.5"},"rustc":"%s","built":"%s"}\n' \
  "$COMMIT" "$(rustc --version)" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > pkg/meta.json
echo "serve: python3 -m http.server 8080 --directory $(pwd)"
