#!/usr/bin/env bash
# A versão mora em três lugares (Cargo.toml do workspace, tauri.conf.json e package.json da interface).
# O Release usa a do tauri.conf.json para a tag; as três têm de bater.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
tauri=$(node -p "require('./crates/nodo-app/tauri.conf.json').version")
ui=$(node -p "require('./crates/nodo-app/ui/package.json').version")
if [[ "$cargo" != "$tauri" || "$tauri" != "$ui" ]]; then
  echo "Versões diferentes: Cargo.toml=$cargo tauri.conf.json=$tauri package.json=$ui" >&2
  echo "Use scripts/bump-version.sh X.Y.Z" >&2
  exit 1
fi
echo "Versão $tauri"
