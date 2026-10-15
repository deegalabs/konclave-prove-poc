#!/bin/sh
# Build the PoC to its own pkg/ (never ui/src/wasm-pkg) and record what was built.
set -eu
cd "$(dirname "$0")"
wasm-pack build --target web --release --out-dir pkg --no-typescript
printf '{"commit":"%s","crates":{"orchard":"0.15.5"},"rustc":"%s","built":"%s"}\n' \
  "$(git rev-parse --short HEAD)" "$(rustc --version)" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > pkg/meta.json
echo "serve: python3 -m http.server 8080 --directory $(pwd)"
