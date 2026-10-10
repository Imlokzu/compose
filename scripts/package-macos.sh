#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
cargo build -p compose --locked
compose_bundle="$PWD/dist/Compose.app"
mkdir -p "$compose_bundle/Contents/MacOS" "$compose_bundle/Contents/Resources"
# Replace the directory entry so an already-running build keeps its original inode.
cp target/debug/compose "$compose_bundle/Contents/MacOS/compose.next"
mv -f "$compose_bundle/Contents/MacOS/compose.next" "$compose_bundle/Contents/MacOS/compose"
cp assets/fonts/OFL.txt "$compose_bundle/Contents/Resources/Lato-LICENSE.txt"
cp assets/fonts/Noto-LICENSE.txt "$compose_bundle/Contents/Resources/Noto-LICENSE.txt"
cp assets/fonts/anton-OFL.txt "$compose_bundle/Contents/Resources/Anton-LICENSE.txt"
cp assets/fonts/bebasneue-OFL.txt "$compose_bundle/Contents/Resources/BebasNeue-LICENSE.txt"
cp vendor/gpui/LICENSE-APACHE "$compose_bundle/Contents/Resources/GPUI-LICENSE.txt"
cat > "$compose_bundle/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>Compose</string>
<key>CFBundleDisplayName</key><string>Compose</string>
<key>CFBundleIdentifier</key><string>app.compose.editor</string>
<key>CFBundleExecutable</key><string>compose</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>0.8.1</string>
<key>CFBundleVersion</key><string>13</string>
<key>CFBundleIconFile</key><string>Compose.icns</string>
<key>NSHighResolutionCapable</key><true/>
<key>LSMinimumSystemVersion</key><string>12.0</string>
<key>NSPrincipalClass</key><string>NSApplication</string>
</dict></plist>
PLIST
swift scripts/make-icon.swift "$compose_bundle/Contents/Resources/Compose.icns"
codesign --force --deep --sign - "$compose_bundle"
printf 'Built %s\n' "$compose_bundle"
