#!/usr/bin/env bash
# Ícone do macOS 26 (Liquid Glass): compila crates/nodo-app/icons/AppIcon.icon (formato do Icon Composer) num
# Assets.car com o actool do Xcode 26 e prepara o build do Tauri para levá-lo no app (Contents/Resources) com
# CFBundleIconName = AppIcon. O icon.icns continua no app: é o que os macOS anteriores mostram.
# Só no macOS, no CI (mexe no Info.plist do checkout). Escreve em $GITHUB_OUTPUT `args` com o --config a somar ao
# `tauri build`; sem Xcode 26 avisa e deixa `args` vazio (o app sai com o ícone antigo, sem falhar o build).
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
app="$root/crates/nodo-app"
out="${GITHUB_OUTPUT:-/dev/stdout}"

# O Xcode mais novo da máquina (o runner do GitHub tem vários; o .icon pede o 26 ou mais).
xcode="$(ls -d /Applications/Xcode_2[6-9]*.app /Applications/Xcode-2[6-9]*.app 2>/dev/null | sort -V | tail -1 || true)"
[ -n "$xcode" ] && export DEVELOPER_DIR="$xcode/Contents/Developer"
# `actool --version` responde um plist; a versão curta vem na linha depois da chave.
ver="$(xcrun actool --version 2>/dev/null | grep -A1 short-bundle-version | tail -1 | sed 's/[^0-9.]//g' || true)"
major="${ver%%.*}"
if [ -z "$major" ] || [ "$major" -lt 26 ]; then
  echo "::warning::actool $ver não compila .icon (precisa do Xcode 26): o app sai com o icon.icns"
  echo "args=" >> "$out"
  exit 0
fi

build="$app/icons/mac"
rm -rf "$build" && mkdir -p "$build"
xcrun actool "$app/icons/AppIcon.icon" --compile "$build" \
  --app-icon AppIcon --include-all-app-icons --enable-on-demand-resources NO \
  --platform macosx --target-device mac --minimum-deployment-target 10.15 \
  --output-partial-info-plist "$build/partial.plist" \
  --output-format human-readable-text --notices --warnings --errors
[ -f "$build/Assets.car" ] || { echo "::error::o actool não gerou o Assets.car"; exit 1; }

/usr/libexec/PlistBuddy -c "Delete :CFBundleIconName" "$app/Info.plist" 2>/dev/null || true
/usr/libexec/PlistBuddy -c "Add :CFBundleIconName string AppIcon" "$app/Info.plist"
cat > "$build/tauri.icon.conf.json" <<JSON
{ "bundle": { "macOS": { "files": { "Resources/Assets.car": "$build/Assets.car" } } } }
JSON
echo "Ícone Liquid Glass: $build/Assets.car (actool $ver)"
echo "args=--config $build/tauri.icon.conf.json" >> "$out"
