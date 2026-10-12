# Android milestone validation · 2026-10-12

The initial mobile APK is native Kotlin/Jetpack Compose + the shared Rust engine.
It is a foundation release with real editing, not full desktop feature parity.

## Verified

- Android 15/API 35 ARM64 emulator: phone at 1080 × 1920 / density 420,
  and tablet layout at 1600 × 2560 / density 240. The emulator is hardware
  accelerated; these are virtual layouts, not physical-device performance results.
- Four offline device scenarios passed in phone and tablet layouts:
  native multiline input and UTF-16 range selection/formatting (only “café” bold,
  surrounding emoji/Unicode text retained), Undo/Redo and activity recreation;
  editable photo preset, shape insertion, local image insertion/proportions,
  exact native file roundtrip and PNG; Word résumé/DOCX, page actions and
  disabled canvas figures; touch movement, circle resize, pinch zoom preserving
  document content and selection. The online scenario is skipped by default.
- The explicit live device scenario passed: actual Android Properties → Remove
  background button, shared Render service, validated transparent preview up to
  800px, unchanged photo bounds and one-step Undo to the original. The only
  uploaded source was the bundled fictional portrait.
- Device screenshots reviewed: phone text selection, photo résumé/cutout,
  shape controls and tablet sidebar. Visual checks caught and fixed native canvas
  overpainting the mode bar/sidebar, startup buttons remaining disabled, pinch
  clearing selection and emoji missing from the Parley document preview.
- Optimized R8 APK installed and launched. Real Android storage-access-framework
  Save created a `.compose` file in Downloads. The same file opened/validated
  through the desktop Rust engine. Force-stop/relaunch recovered its local draft.
- APK signature checked; API 26 minimum / API 35 target, ARM64 only.
  Native library LOAD segments use 0x4000 alignment (16KB).
- 99 workspace tests and strict workspace/all-targets Clippy passed.
  Android's headless Rust dependency tree has no GPUI.
- Desktop native regression passed after the renderer separation: native font
  faces/Light/Black controls, basic editing/history, figures and PNG/print output.

## Release and limits

- Artifact: `dist/Compose-Android.apk`, version 0.15.0 / code 1; approximately
  31 MiB. Bundled fonts, original presets and the native engine work offline.
  Background removal requires the existing shared online service.
- The APK uses the local Android development signing identity. A Play Store
  release, production signing ownership and store publication were not performed.
- Native keyboard/selection display and exported document glyphs can differ in
  platform emoji appearance; source text, ranges and document layout persist.
- Web image search, native Android printing, rotation, freeform paths,
  multi-selection/group scaling, advanced mobile links/style controls and broader
  accessibility remain further milestones. Canvas groups remain in saved files.
- Tests use an Android 15 emulator; physical old-phone performance, Android 8–14
  runtime behavior and 32-bit devices are not certified. The delivered APK is ARM64.
- One observed optimized emulator launch was 0.6 seconds and memory PSS around
  116 MiB in a simple document. These are observations on this host, not promised
  minimum-hardware benchmarks.
- Existing desktop windows/documents and credentials were preserved. A dedicated
  Compose QA emulator was used for final verification. Source contains no provider
  secrets or signing keys; local SDK/build paths remain ignored.

Evidence lives in the ignored `artifacts/` directory: android-device-tests.log,
android-tablet-tests.log, android-live-background-test.log, android-package.log,
android-final-package.log, android-workspace-tests.log,
android-workspace-clippy.log, android-desktop-regression.log,
android-file-interoperability.log and android/phone + android/tablet screenshots.
