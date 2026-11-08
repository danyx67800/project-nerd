#!/usr/bin/env bash
# Build a signed ad-hoc Project Nerd.app (and optionally a universal .dmg).
set -euo pipefail
cd "$(dirname "$0")/.."

APP_NAME=$(sed -n 's/.*pub const APP_NAME: &str = "\(.*\)";/\1/p' crates/nerd-core/src/branding.rs)
DEFAULT_BUNDLE_ID=$(sed -n 's/.*pub const DEFAULT_BUNDLE_ID: &str = "\(.*\)";/\1/p' crates/nerd-core/src/branding.rs)
MIN_MACOS=$(sed -n 's/.*pub const MIN_MACOS_VERSION: &str = "\(.*\)";/\1/p' crates/nerd-core/src/branding.rs)
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
SHORT_VERSION=${VERSION%%-*}
BUNDLE_ID=${BUNDLE_ID:-$DEFAULT_BUNDLE_ID}
BUILD_NUMBER=${BUILD_NUMBER:-$(date +%Y%m%d%H%M)}
CODESIGN_IDENTITY=${CODESIGN_IDENTITY:--}

UNIVERSAL=0
DMG=0
for arg in "$@"; do
  case "$arg" in
    --universal) UNIVERSAL=1 ;;
    --dmg) DMG=1 ;;
    --help|-h)
      echo "uso: $0 [--universal] [--dmg]"
      exit 0
      ;;
    *) echo "opzione sconosciuta: $arg" >&2; exit 2 ;;
  esac
done

if [ -z "$APP_NAME" ] || [ -z "$BUNDLE_ID" ]; then
  echo "impossibile leggere il branding da crates/nerd-core/src/branding.rs" >&2
  exit 1
fi

PROFILE_DIR=target/release
if [ "$UNIVERSAL" -eq 1 ]; then
  for target in aarch64-apple-darwin x86_64-apple-darwin; do
    if ! rustup target list --installed | grep -qx "$target"; then
      echo "installo il target $target…"
      rustup target add "$target"
    fi
  done
  cargo build --release --target aarch64-apple-darwin
  cargo build --release --target x86_64-apple-darwin
else
  cargo build --release
fi

DIST=dist
APP="$DIST/$APP_NAME.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

if [ "$UNIVERSAL" -eq 1 ]; then
  lipo -create \
    target/aarch64-apple-darwin/release/project-nerd \
    target/x86_64-apple-darwin/release/project-nerd \
    -output "$APP/Contents/MacOS/$APP_NAME"
else
  cp "$PROFILE_DIR/project-nerd" "$APP/Contents/MacOS/$APP_NAME"
fi
chmod +x "$APP/Contents/MacOS/$APP_NAME"

# Icon: generated once from scripts/make_icon.py, then expanded to .icns.
if [ ! -f assets/icon.png ]; then
  python3 scripts/make_icon.py
fi
ICONSET=$(mktemp -d)/AppIcon.iconset
mkdir -p "$ICONSET"
for size in 16 32 128 256 512; do
  sips -z "$size" "$size" assets/icon.png --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
  sips -z "$((size * 2))" "$((size * 2))" assets/icon.png --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/AppIcon.icns"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>$APP_NAME</string>
    <key>CFBundleDisplayName</key><string>$APP_NAME</string>
    <key>CFBundleExecutable</key><string>$APP_NAME</string>
    <key>CFBundleIdentifier</key><string>$BUNDLE_ID</string>
    <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>$SHORT_VERSION</string>
    <key>CFBundleVersion</key><string>$BUILD_NUMBER</string>
    <key>CFBundleIconFile</key><string>AppIcon</string>
    <key>LSMinimumSystemVersion</key><string>$MIN_MACOS</string>
    <key>NSHighResolutionCapable</key><true/>
    <key>LSApplicationCategoryType</key><string>public.app-category.music</string>
    <key>NSSupportsAutomaticGraphicsSwitching</key><true/>
</dict>
</plist>
PLIST
printf 'APPL????' > "$APP/Contents/PkgInfo"

# iconutil and the file provider can leave metadata that codesign rejects.
/usr/bin/xattr -cr "$APP" 2>/dev/null || true
dot_clean -m "$APP" 2>/dev/null || true
find "$APP" -exec /usr/bin/xattr -c {} \; 2>/dev/null || true
/usr/bin/xattr -d com.apple.FinderInfo "$APP" 2>/dev/null || true
codesign --force --sign "$CODESIGN_IDENTITY" "$APP"
codesign --verify --deep --strict "$APP"

ZIP="$DIST/$APP_NAME.zip"
rm -f "$ZIP" "$ZIP.sha256" "$ZIP.sha256.sig"
ditto -c -k --sequesterRsrc --keepParent "$APP" "$ZIP"
shasum -a 256 "$ZIP" | awk '{print $1}' > "$ZIP.sha256"

if [ -n "${OTA_KEY:-}" ]; then
  cargo run --release -q -p nerd-tools -- sign --key "$OTA_KEY" --file "$ZIP.sha256" --out "$ZIP.sha256.sig"
else
  echo "OTA_KEY non impostata: firma OTA saltata (la build non è distribuibile)." >&2
fi

if [ "$DMG" -eq 1 ]; then
  DMG_PATH="$DIST/$APP_NAME.dmg"
  STAGE=$(mktemp -d)
  cp -R "$APP" "$STAGE/"
  ln -s /Applications "$STAGE/Applicazioni"
  rm -f "$DMG_PATH"
  hdiutil create -volname "$APP_NAME" -srcfolder "$STAGE" -ov -format UDZO "$DMG_PATH" >/dev/null
  rm -rf "$STAGE"
  echo "dmg: $DMG_PATH"
fi

echo "app: $APP"
echo "zip: $ZIP"
echo "sha256: $ZIP.sha256"
