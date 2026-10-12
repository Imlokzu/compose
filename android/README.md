# Compose for Android

Native Kotlin/Jetpack Compose UI with the existing Rust engine through JNI.
The mobile editor shares desktop icons, document formats, fonts and components.
Complete desktop feature parity remains future work.

## Build

On the current macOS development host, install JDK 17, the Android SDK and NDK
28.2.13676358. Install Rust's `aarch64-linux-android` target through rustup.
Set `ANDROID_HOME` if the SDK is outside `~/Library/Android/sdk`.

```sh
rustup target add aarch64-linux-android
./scripts/package-android.sh
adb install -r dist/Compose-Android.apk
```

The initial APK supports 64-bit ARM devices running Android 8.0/API 26 or newer.
Native ELF segments are aligned for 16KB pages. The release APK is optimized with
R8 and signed with the local development key; store publishing and an owner release
signing identity remain separate work. Do not share the private signing key.

## Working editor

- Real editable presets, Compose/Word-compatible document modes and multiple pages.
- Shared `.compose` save/open through Android's file picker, preserving desktop
  text, shape geometry, original images, styles and nesting. A local draft is saved
  after edits and can be continued after process restart.
- Native multiline keyboard/IME input, Android text selection and clipboard.
  Bold, italic, underline, font, real bundled weights, size, color and hyperlinks
  apply to the selected passage or whole box. Font menus preview actual faces;
  mixed ranges retain unrelated styles. Paragraph alignment is editable.
- Rectangle, Circle, Oval, Triangle, Diamond, Pentagon, Hexagon, Star, Arrow, Line
  and Frame. Tap to select, drag selected objects, resize handles, two-finger pan
  and pinch zoom. Circles and Scale-mode images keep their proportions.
- Desktop line icons, labeled touch tools, two/four-column galleries with previews
  rendered from real editable presets, blue active states and subtle panel dividers.
- Phone Layers/Properties sheets, tablet Layers sidebar and a right inspector on
  wide tablets. Nested layers expand/collapse, hide/show, lock/unlock and reorder.
  Assets inserts desktop Title, Callout, Quote, Stat, Card and Divider components as
  ordinary editable nested objects, in one undo step.
- Numeric position/size, image Scale/Stretch and Bigger/Smaller, color, opacity,
  Square/Soft/Round corners, strokes, shadow, page alignment, duplicate and delete
  have real engine commands. Image dimensions retain proportions in Scale mode.
- Original local image insertion and the existing keyless shared background-removal
  service. Cutouts preserve geometry and Undo. No provider credential is bundled.
- PNG export and editable DOCX export in Word-compatible mode. Native printing,
  web image search, rotation, multi-selection/group scaling and freeform paths are
  not exposed in the Android UI yet.

The page preview uses Rust/Parley glyph layout and the same raster renderer as PNG
output. Android reads its system Arabic/CJK/Devanagari fallback fonts when present.
The shared renderer uses bundled Noto bitmap emoji because Swash does not paint
Android 15's system COLRv1 emoji font. Font licenses are included in the APK.

`python3 scripts/android-icons.py` regenerates Android vectors from the canonical
desktop SVGs in `assets/icons`. No icon library or network font download is needed.

## Emulator checks

```sh
android/gradlew -p android connectedDebugAndroidTest
# Explicit real service check; uploads only the bundled fictional portrait:
android/gradlew -p android connectedDebugAndroidTest \
  -Pandroid.testInstrumentationRunnerArguments.class=app.compose.mobile.EditorDeviceTest#liveBackgroundRemovalRetainsPhotoAndUndo \
  -Pandroid.testInstrumentationRunnerArguments.liveBackground=true
```

The ordinary suite exercises native selection/formatting, Undo/Redo, recreation,
photo presets, local image import, native files/exports, document modes/pages,
touch movement/resizing and pinch selection preservation. The online check is
opt-in. The validation report records actual emulator devices, screenshots,
release checks and limits; an emulator does not certify older physical hardware.
