#!/usr/bin/env bash
# Builds the garden's site into dist/, a self-contained static site. Bake the garden first:
#   cargo run --release -p bake -- --garden --output assets/garden.solstice
#
# The wasm-bindgen tool must match the wasm-bindgen library exactly, so the matching
# version is installed inside target/, leaving any global installation alone.
set -euo pipefail
cd "$(dirname "$0")/.."

version=$(grep -A1 '^name = "wasm-bindgen"$' Cargo.lock | sed -n 's/^version = "\(.*\)"$/\1/p')
tools=target/tools
bindgen="$tools/bin/wasm-bindgen"

if ! "$bindgen" --version 2>/dev/null | grep -q "$version"; then
  echo "installing wasm-bindgen-cli $version..."
  cargo install wasm-bindgen-cli --version "$version" --root "$tools" --locked
fi

# The garden is a place to walk, with no game to play, so it needs only the plain viewer, and
# its own page is the site's front page.
rm -rf dist
mkdir -p dist/assets
cargo build --release --target wasm32-unknown-unknown -p viewer --lib --no-default-features
"$bindgen" --target web --no-typescript --out-dir dist/pkg \
  target/wasm32-unknown-unknown/release/solstice_viewer.wasm
cp apps/viewer/web/garden.html dist/index.html
# The scene and the heights its page walks on. Hard links keep the scene from taking disk space
# twice.
for file in assets/garden.solstice assets/garden.walk; do
  # None in CI: there, they come from the release scene.txt names.
  [ -e "$file" ] || continue
  ln -f "$file" "dist/assets/$(basename "$file")" 2>/dev/null || cp "$file" "dist/assets/"
done

echo "built dist/. To try it: python scripts/serve.py"
