#!/usr/bin/env bash
# Sobe a versão nos três lugares de uma vez e atualiza o Cargo.lock. Uso: scripts/bump-version.sh 0.2.0
set -euo pipefail
cd "$(dirname "$0")/.."
v="${1:?uso: scripts/bump-version.sh X.Y.Z}"
[[ "$v" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "versão inválida: $v" >&2; exit 1; }
sed -i.bak "0,/^version = \".*\"/s//version = \"$v\"/" Cargo.toml && rm Cargo.toml.bak
for f in crates/mapist-app/tauri.conf.json crates/mapist-app/ui/package.json; do
  node -e "const f='$f',j=require('./'+f);j.version='$v';require('fs').writeFileSync(f,JSON.stringify(j,null,2)+'\n')"
done
cargo update --workspace --quiet
scripts/check-version.sh
