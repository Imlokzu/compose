# Compose

A lightweight alternative to Microsoft Word, with canvas design tools for
reports, homework, posters and everyday documents.

**Write like a document. Design like a canvas.**

Compose combines rich text editing with movable images, shapes, layers and
groups in a calm native interface. Built in Rust with GPUI and Parley, without
Electron or webviews.

![Compose's native editor with multiple selection and design controls](docs/screenshots/editor.png)

Compose is in active development. macOS is currently tested; Windows/Linux
packaging and interaction checks are still pending. It uses its own editable
`.compose` files; DOCX import/export is not implemented yet.

## Run

Install Rust (currently validated with 1.98.1). On macOS, install the Xcode
Command Line Tools with `xcode-select --install`; the build uses Swift only for
the small native printing adapter.

```sh
cargo run -p compose
# Open a saved document:
cargo run -p compose -- /path/to/document.compose
```

On macOS, create a local development app bundle:

```sh
./scripts/package-macos.sh
open dist/Compose.app
```

The bundle includes fonts and a vector app icon. It is locally ad-hoc signed,
not a notarized distribution. macOS is the platform tested in this workspace.
Linux/Windows support still needs platform-specific builds and interaction QA.

## Working features

- Six bundled typefaces with actual family previews, live search, real weights,
  italic variants where available, and seven typography presets.
- Rich text formatting applies only to the highlighted passage. Mixed sizes,
  family, weight, color, spacing, line height and alignment remain editable.
- Continuous HSV color picker with hex/RGBA and alpha input, 32 palette colors,
  and colors already used in the document. One color drag is one undo step.
- Assets library: Title, Callout, Pull quote, Key number, Feature card and Divider.
  Components insert as editable nested frames/text/shapes; they are independent
  layouts, not linked instances.
- Visual object fills, rounded corners, strokes and soft drop shadows, persisted
  in `.compose` files. Images retain original bytes and proportionate Scale mode.
- Layers tree with visibility, locks, nesting, renaming, ordering and duplication.
- Shift-click and marquee selection, shared movement/nudging, batch clipboard,
  grouping/ungrouping, alignment and even spacing for several objects.
- Multiple pages, page-size presets/orientation, editable document title, preview,
  pan/zoom, snapping, inspector fields and native menus.
- Undo/redo, native file dialogs, atomic saves and unsaved-change prompts.
- Export editable `.compose` copies and current-page PNGs at 1×, 2× or 300 DPI.
  Print all pages through the macOS print dialog, including its Save as PDF option.
- Copy/cut/paste layers and nested frames across documents in the same running
  app. Pasted images keep their original bytes and attribution.
- Copy highlighted passages with inline formatting; paste external text onto the
  canvas to create an editable text box.
- Replace images from your computer, web search or Commons while keeping their layer name,
  position, width and appearance. Scale adjusts height to the new proportions;
  Stretch keeps both dimensions.

Click the document title to rename it. The **Assets** tab holds editable layout
blocks. The typeface field opens the font picker; the color row opens the palette
and continuous picker. Use the eye/lock icons beside each layer. The preview
triangle hides editing controls; Escape returns to the editor.

Double-click a blank area to write. **T** also lets you click and drag within an
existing text box to highlight a passage. Double-click a word to select it. Bold,
italic, underline, font size, family, and color apply only to that selection;
the floating toolbar stays with the selected text. The inspector shows `Mixed`
when selected characters have different property values.

Highlight a passage to open its centered text menu. Choose fonts, weights, exact
sizes or presets, text styles, bold/italic/underline, color or clear formatting
right beside the selection. Font search is focused automatically; Enter applies
the first match. Styles include Title, Heading, Subheading, Body, Caption, Quote
and Code. All changes apply to the highlighted passage. Run native interaction
checks with `cargo run -p compose -- --ui-smoke --text-menu`.

Check every bundled native font face and the Light/Black controls with
`cargo run -p compose -- --ui-smoke --font-weights`. The renderer maps typographic
family names consistently between Parley and GPUI, including extended weights
and italic variants.

Images default to **Scale**, which preserves their original proportions with
any handle or width/height field. **Smaller** and **Bigger** uniformly scale by
10% around the center. Choose **Stretch** for independent dimensions; choosing
Scale again restores the original aspect ratio. These edits retain original
image bytes and are undoable.

**Corners** provides Square, Soft, Round and an exact radius field. Drag the
small round handle inside a selected image or shape to adjust its corners.
The eight square handles resize the element; the cursor indicates their direction.

Use V/T/R/F for select/text/rectangle/frame. Shift constrains other objects;
Alt resizes about the center or bypasses move snapping. Space+drag or the wheel
pans. Cmd/Ctrl+wheel zooms; clicking the zoom percentage fits the page.
On macOS, pinch the touchpad to zoom; two-finger scrolling pans. Pinch and
modifier-scroll zoom stay anchored near the pointer.
Cmd/Ctrl+S saves, Shift+Cmd/Ctrl+S saves as, O opens, N creates a blank document,
Z undoes, Shift+Z redoes, D duplicates. B/I/U format selected text. Escape leaves
text editing. Numeric inspector fields accept typed values; Enter commits and
Escape cancels. Colors accept six-digit hex. Double-click a layer to rename it.

Clipboard currently handles text. Object duplication uses Cmd/Ctrl+D. Text is
edited in independent boxes; automatic flow between pages, lists, tables,
rotation, vector PDF/HTML/DOCX export, collaboration and cloud features are later work.
System fallback typography, screen-reader support and large-image performance
need more validation. IME range handling is tested; all OS/language candidate
window behavior is not yet certified.

## Verify

```sh
cargo test --workspace
cargo run -p compose -- --smoke-test
cargo run -p compose -- --ui-smoke
cargo run -p compose -- --ui-smoke --refinements
cargo run -p compose -- --ui-smoke --polish
```

The native smoke test sends events inside Compose; it doesn't move the real
pointer or change the clipboard. It writes `artifacts/native-smoke.compose`.
Use `--ui-smoke-hold` to leave its editable fixture open for visual inspection.

Run `cargo run -p compose -- --ui-smoke --workflows` for clipboard and replacement
regressions. The checks leave your OS clipboard untouched. Object transfer between
separate app instances remains unsupported; readable text is the fallback. See
`docs/FEATURE_GAPS.md` for the prioritized remaining features.
The refinement checks use `artifacts/refinement-checks/`, so the original sample
is not overwritten. Add `--show-text-selection` with `--ui-smoke-hold --refinements`
to inspect the selected-word toolbar. Native screenshots are saved as
`artifacts/editor-images-v2.png` and `artifacts/editor-text-v2.png`.

Architecture is in `ARCHITECTURE.md`; design decisions and the original image
references are in `DESIGN.md` and `brand/`. Font licenses and the small GPUI API
bridge are documented under `assets/fonts/` and `vendor/README.md`.

The 0.3 showcase and native screenshots are under `artifacts/polish/`. Use
`--ui-smoke-hold --polish --show-fonts` or `--show-colors` for visual inspection,
and `--compact` for the minimum window size. These tests stay inside Compose
and do not move the system pointer or change the clipboard. Final visual QA
uses the packaged application because macOS can defer presentation of fully
occluded background windows.

Release limits: synchronized component instances, selection/group scaling, rotation, vector
path editing, linked text pagination, further export/import formats and multi-platform release
certification remain future milestones. This is a local macOS development build,
not full Figma parity. Review availability is recorded in the validation artifacts.

## Add images from your computer or the web (0.6)

Click **Image** in the toolbar or Insert area to open the native image picker.
**My computer** opens the system file chooser. **Search online** searches
the full web through Bing Images inside Compose: enter a phrase and press Enter,
or use a suggestion. **Commons** remains a separate optional source. Select a
thumbnail, inspect its title and originating website, and click
**Insert image**. More images loads the next results page.

The selected original raster image is embedded in the `.compose` file; source,
creator and license metadata, when provided, are saved with it and shown in the inspector.
Search needs an internet connection; an inserted image stays available offline.
Source and license links open only when clicked. Web search requires no API key;
unknown usage rights remain unknown. If a website blocks its original download,
**Insert preview** explicitly uses the cached lower-resolution thumbnail. Search
errors also offer **Open search in browser**. Bing's public result markup can change;
Compose reports provider failures instead of silently switching to a catalog.

Search and image decoding run on background workers. Closing the picker cancels
pending UI effects; a newer search rejects older replies. The picker shows loading,
empty, error/retry and preview-failure states. Supported originals are JPEG, PNG,
WebP, GIF and BMP, limited to 64 MB / 32 megapixels. GIFs currently insert their
first frame. Images retain the existing Scale/Stretch and corner controls.

Explicit integration checks (these access Bing and Commons):

```sh
cargo run -p image_search --example live
cargo run -p compose -- --image-picker-test
# Leave the results visible for inspection:
cargo run -p compose -- --image-picker-hold --compact
```

Offline tests use recorded search metadata. Live tests exercise native tab and
query controls, thumbnails, original download, insertion, attribution, undo,
reopening, local loading, invalid images and stale-request cancellation.

## Export and print (0.7)

Click **Export** in the top bar, or press **⇧⌘E**. **Compose file** exports a fully
editable copy with every page and embedded image. The current document's save
location and saved/unsaved state are retained. **Save** still updates the working
file, and **Save As** changes its location.

**Export PNG** writes the current page without editor controls. Choose 1× (72 DPI),
2× (144 DPI), or 300 DPI; the menu shows the pixel dimensions. PNGs keep text
layout, mixed fonts, image proportions, rounded corners, layer order, opacity,
strokes and shadows. PNG export uses document coordinates and is independent of
zoom or window size. Very large pages can require a lower resolution.

**Print…** or **⌘P** prepares all document pages at 300 DPI and opens the macOS
print dialog. Choose a printer, copies and page range, or use the dialog's
**Save as PDF**. Cancelling leaves the document untouched. Print output currently
uses raster pages; system paper/printer margins may scale the document. Native
printing on Windows/Linux and vector PDF export remain future work.

Both operations run in background workers. They use local snapshots and never
require a network connection. The File menu also exposes the export/print actions.
The bundle includes the small native print adapter; users do not need Swift tools.

```sh
# Native export controls and a two-page print-to-PDF check (no physical printing):
cargo run -p compose -- --ui-smoke --output
# Also open and automatically dismiss the system print panel safely:
cargo run -p compose -- --ui-smoke --output --check-print-dialog
```

## Select and organize layouts (0.8)

Use **Shift-click** on the canvas or in Layers to add/remove objects. Drag a
rectangle from empty space to select several objects; hold Shift to add to the
current selection. **⌘A** selects page objects when you are not editing text.
Drag the selection to move it together. Arrow keys nudge it by 1 pt; Shift+arrows
use 10 pt. Copy/cut/paste, duplicate, delete and layer ordering operate on the
whole selection and keep a single undo step.

The inspector shows selection bounds, six alignment controls and horizontal or
vertical spacing for three or more objects. **Group** / **⌘G** groups unlocked
layers with the same parent. A normal click moves the group; double-click to
edit a child. Escape returns to its group. **Ungroup** / **⇧⌘G** restores the
children and their positions. Groups follow their contents when children move
or text reflows, preserve embedded images and reopen in `.compose` documents.

Grouping keeps selected layers' order and places the group at their frontmost
selected layer. For nonadjacent layers, this can change their stacking relative
to unselected objects between them. Grouping across different parents is currently
disabled. Multiple-selection and group scaling are not yet implemented, so these
selections show movement bounds without resize handles. Individual objects retain
their normal resizing, corners and text formatting controls.

```sh
cargo run -p compose -- --ui-smoke --selection --compact
# Leave the real selection inspector visible:
cargo run -p compose -- --ui-smoke-hold --selection --compact
# Inspect the grouped version:
cargo run -p compose -- --ui-smoke-hold --selection --show-selection-group --compact
```

## Touchpad zoom (0.8.1)

Pinch in or out over the workspace to zoom smoothly between 15% and 400%.
Two-finger scrolling continues to pan, and Ctrl/⌘-scroll also zooms. The document
point near the pointer stays fixed while zoom changes, including multiple input
events before the next redraw. Sidebar/popover gestures do not zoom the page.
Camera changes retain text/object selection, saved state and history.

```sh
# Tests the actual macOS view callback with synthetic AppKit gesture events:
cargo run -p compose -- --ui-smoke --touchpad --compact
```

## License

Compose's original source code is MIT licensed. Vendored GPUI retains its
Apache-2.0 license, and bundled fonts retain their included font licenses.
