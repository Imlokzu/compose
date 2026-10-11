# Compose architecture

Write like a document. Design like a canvas.

## Editable figures

`document::ShapeKind` and `PathCommand` define native figure geometry without
UI dependencies. Shape nodes retain the existing `NodeKind::Shape` representation
and add a defaulted subtype and line direction. Legacy files decode as rectangles.
The native canvas converts the shared path commands into GPUI paths; PNG/print
uses the same geometry with tiny-skia. Hit-testing uses the visible figure rather
than its whole bounding rectangle, with a small selection tolerance for lines.
Rectangle quads remain the fast path; other closed figures use vector fills and
centered strokes. Polygon radius rounds each corner. Native polygon shadows use
bounded layered paths; exported shadows use the existing raster Gaussian blur.

Circle creation, numeric edits and every resize handle constrain its ratio to 1;
validation rejects a stretched circle. Line direction is captured from the draw
gesture, and thickness is independent of its endpoint rectangle. The compact
picker shares toolbar/inspector choices, drag payloads, lock guards and snapshot
history. Shapes remain Compose-only; Word mode retains its existing restriction.

## Clipboard and image replacement

`editor::ObjectClipboard` holds detached subtrees and only their referenced assets.
The roots use page coordinates, descendants stay relative. Paste validates a
candidate document, assigns fresh IDs and commits one snapshot. `app::clipboard`
routes fields, formatted passages and object selection separately. Rich-text
metadata travels with readable text; object metadata is an instance token for
retained `Arc` bytes, avoiding large image serialization on the UI thread. Object
data survives document switches; other instances/apps receive text.

The image picker stores an optional replacement target alongside its cancellation
generation. Downloads remain in the background. Completion rechecks existence,
kind and locks before changing the asset/source and proportional height in one
checkpoint. Layer identity, position, width and appearance remain stable. Closing
the modal clears the target and invalidates callbacks. The native format's model
and version remain compatible.

## Implemented boundaries

```text
app                 native window, GPUI panels, gestures, platform text input, dialogs
 ├─ renderer        Parley layout and caret/selection geometry; GPUI glyph painting
 │   ├─ text        attributed UTF-8 text, character styles, Unicode editing
 │   └─ document    pages, objects, hierarchy, assets, validation
 ├─ editor          snapshots, selection, layer mutations, reparenting, hit-testing
 │   └─ document
 ├─ layout          snapping geometry in document coordinates
 │   └─ document
 ├─ export          document-page raster output, shared Parley glyph layout
 └─ file_format     versioned ZIP serialization, bounded reads, atomic save
     └─ document
```

Everything below `app` and `renderer` is independent of GPUI. History currently
lives in `editor`; gestures and reusable UI widgets live in focused app modules.
Separate `history`, `tools`, and `ui` crates are unnecessary until they have a
second consumer. No empty placeholder crates. Export has its own output layer.

## Document model

Units are points (1/72 inch), never physical pixels. A4 is 595.28 × 841.89 pt.
A document has pages; each page owns a back-to-front list of objects. Frame
children have positions relative to the frame. Reparenting preserves absolute
position and rejects cycles. Invisible or locked nodes are excluded from hits.
Image originals are stored in an asset map with `Arc<Vec<u8>>`; history snapshots
share the bytes. Text, image, rectangle and frame are real editable node kinds.

The model is serializable and contains no selection, focus, camera, caches,
input composition or GPU resources. Geometry, text span boundaries, IDs,
nesting depth and image references are validated before save and after load.

## Text and Parley

`text::RichText` owns content, a default style, normalized nonoverlapping style
spans and block alignment. Edits operate on UTF-8 boundaries; deletion uses
Unicode graphemes. GPUI's platform input protocol uses UTF-16, with explicit
conversion at the adapter boundary. Native composition tracks a marked range.
Typing coalesces into history; a gesture gets one checkpoint.

Parley 0.11.1 shapes **all document text**, including mixed sizes and families in
a line. It supplies wrapping, alignment, bidi ordering, caret movement,
hit-testing and selection rectangles. Layout is cached by content/styles/width
at zoom 1.0. Zoom scales glyph positions and raster size, leaving line breaks
unchanged. Text boxes expand for changed text that needs more space; width
controls wrapping, and the height can retain additional layout space.

The exact Parley glyph IDs are painted through GPUI. Lato and Noto Serif static
regular/bold/italic/bold-italic fonts are bundled, licensed, and registered in
both systems. System fallback fonts are used for other scripts and emoji;
their cross-platform identity and variable-font settings need further work
before guaranteeing portable typography outside the bundled families.

GPUI 0.2.2 is vendored with three documented API visibility changes, described
in `vendor/README.md`. There is no replacement shaping pass through GPUI.

## Coordinates and interactions

`screen = page_origin + document_point * zoom`; pan is stored in logical screen
pixels. The page is centered horizontally inside the viewport. GPUI then handles
the display's physical scale factor. All object hit-testing happens in document
coordinates, in reverse paint order. Resize handles use constant screen targets.
Nested nodes accumulate parent transforms for drawing and hit-testing.

The app's gesture state distinguishes selection, text selection, move, resize
and pan. Move snapping considers page edges, center, margins and object bounds.
Shift constrains movement/proportions; Option/Alt bypasses move snapping and
resizes about the center. Wheel pans; Ctrl/Cmd+wheel zooms around the pointer.
Text has single/double/triple-click caret, word and whole-box selection, visual
arrow motion, shift extension, multiline input and text clipboard operations.

## Native storage

`.compose` is a ZIP container with `manifest.json`, `document.json`, and
`assets/<content-hash>`. The manifest records format version 1. Original image
bytes are embedded. DOCX is not involved. Saving uses a synced sibling temp file
and atomic replacement. Failed validation leaves the previous file untouched.
Reads reject future versions, duplicate entries, excessive sizes and invalid
models. The UI keeps the current document when opening fails and prompts for
unsaved changes before New, Open, or Close.

## Verification and remaining risk

Engine tests cover Unicode/style edits, mixed-size Parley layout, caret mapping,
history, nesting, snapping, and native file roundtrips/corruption. `--smoke-test`
exercises the engine integration. `--ui-smoke` opens the native app and dispatches
GPUI input events through the real handler chain. It tests text insertion,
selection, formatting, UTF-16/composition ranges, inspector typing, text/image
dragging and resizing, history, pointer-anchored zoom and save/reopen.

This is the first vertical slice, not a completed Word replacement. Known next
milestones: flowing text across pages, lists/tables, mixed paragraph styles,
full keyboard focus traversal and screen-reader integration, stronger long-file
performance, system-font portability, rotation/effects, and further export/import.
Parley enables reusing a layout for export; identical PDF output still requires
an implemented and tested exporter. macOS has been exercised locally; Windows
and Linux builds and native input require their own CI/QA before release claims.

## Image and selection refinement (0.2)

`Node::image_sizing` persists Scale or Stretch; serde defaults older files to
Scale. Source pixels remain untouched. Scale renders a fitted source rectangle
and couples both dimensions during pointer/numeric edits. Returning to Scale
restores source proportions. `layout::resize_rect` handles corner/edge anchors,
minimum dimensions and center resizing without involving GPUI. Radius is clamped
to half the shorter side when rendered; its direct-manipulation handle is one
history transaction. Scaling buttons also scale the radius.

The Text tool now hit-tests existing text before inserting. Double-click selects
a word from object selection, and byte ranges stay intact while the inspector
or floating controls have focus. Floating controls occlude the canvas so clicks
do not reset the selection. Inspector and floating fields identify their input
source separately; only the active field draws a caret and provides IME bounds.
Mixed style values are displayed rather than flattening the text model.

The native refinement test includes actual mouse selection and floating-toolbar
clicks, range-only font/color changes, text replacement, all eight non-square
image handles, linked width/height edits, scaling, stretching, radius dragging,
undo, original-pixel retention and complete file roundtrips.

## Product polish (0.3)

The editor crate now owns the reusable component builders. They create ordinary
hierarchical nodes with unique IDs in one snapshot transaction; there is no
second component-only renderer or linked-instance model. Lock inheritance follows
parent IDs; canvas hit-testing skips locked/hidden subtrees. Layer toggles are
undoable. UI edits to locked selections are guarded at input/property boundaries.

`Appearance` adds stroke color/width and shadow parameters with serde defaults,
finite/range validation, and native ZIP roundtrip tests. These fields are painted
by the GPUI canvas; original image bytes are unchanged.

`text::palette` contains independent HSV/RGBA math. The UI owns a transient picker
with target, HSV state and drag transaction flag. Text-color mutation uses the same
range formatter as the inspector. History checkpoints once on the first changed
color of a drag, then subsequent moves amend that transaction. Popup/font-search/
preview state is transient and never serialized.

`renderer::fonts` lists six bundled static families and the actual available faces.
Formatting normalizes requested weights/styles to supported faces before storing
them, avoiding synthetic-weight differences between Parley and GPUI. Existing
unknown font names are retained rather than relabeled. Tests verify Parley actually
resolves every offered font family. Font licenses and sources are under assets/fonts.

The Parley-to-GPUI glyph bridge uses the font's typographic family name (name ID
16), falling back to legacy name ID 1 only when the preferred name is absent.
Lato Light/Black and several Noto Serif faces have legacy family names that differ
from their native registration name. Resolving those legacy names silently chose
a system font and painted Parley glyph IDs through the wrong face. Native checks
now verify all 20 bundled weight/style combinations resolve distinct faces with
identical glyph indices in Parley and GPUI.

Native test coverage for the polish includes filtered font-picker clicks, selected
text colors, RGBA/alpha fields, single-step drag undo, asset insertion, lock guards,
visibility, preview immutability, page setup and styled-component roundtrips.

## Native image picker (0.4)

`image_search` is an independent HTTP/provider adapter returning normalized search
results. It uses the anonymous Wikimedia Action API and a reqwest blocking client
on GPUI background workers, with timeouts, bounded JSON/image reads, MIME/size
filters, and HTTPS host checks on both initial requests and redirects. Only
Commons and Wikimedia media hosts are fetched. Metadata HTML is rendered as plain
text. No API secret, webview or document content is sent to the provider.

`app/image_picker` owns transient search/query/selection and request state. A
monotonic generation plus cancellation token invalidates replies after closing,
new queries or changing documents. Thumbnail concurrency is four; all network
requests and image decoding execute away from the UI thread. Old responses cannot
populate a reopened picker or insert into a different document. Local-file import
uses the same decoded-image insertion path and one history checkpoint.

`Node::image_source` is optional with a serde default, preserving old documents.
It stores provider, source/original URLs, author, license URL/label and credit.
Original image bytes remain in the content-hashed asset store. Source links are
explicit user actions and are checked before opening, including on imported files.

## Full web image search (0.6)

`image_search::Provider::Web` is the default. Ordinary public Bing Images requests
return HTML containing structured image metadata; `scraper` parses this data into
the existing Rust result model. No HTML is rendered in the app. `Provider::Commons`
retains Wikimedia as a separately labeled source. The native picker reuses its
grid, background workers and cancellation generation.

Originals/thumbnails can come from public HTTPS sites. Each redirect validates
the address and pins resolved public IPs, retaining byte/pixel/timeout bounds.
Result metadata cannot cause private-network downloads. Unknown web dimensions
are checked after decoding; unknown licensing stays empty. Provider/source details
are embedded with the original. An explicit preview fallback embeds the cached
thumbnail and marks its reduced resolution in credit.

Bing's public metadata is not a guaranteed API. Blocked/changed pages produce an
actionable error rather than an empty success. Browser search is an explicit
fallback; source selection never silently changes the search provider.

## Export and native printing (0.7)

`export::PageRenderer` reads validated document snapshots in points. It uses the
same Parley layout/font data and image-fitting geometry as native paint. Swash
rasters those positioned glyph IDs, including available color-font bitmaps;
tiny-skia composites shapes, images, text clips, opacity, inner strokes and rounded
corners. Software Gaussian shadows approximate the GPUI shadow backend. The
exporter requires no window, GPU device, selection, caret or camera state.

PNG output is bounded to 32 megapixels and atomically replaced, with physical DPI
metadata. `app::output` owns the compact Export popup and native destination
dialogs. The editable-copy action uses `file_format::save`; exporting leaves the
working path, saved snapshot and history unchanged. The default PNG scope is the
current page. Save/Save As retain their original semantics.

`app::printing` renders all pages at 300 DPI into a private temporary directory.
The macOS build embeds a small compiled AppKit adapter, so installed users need
no developer tools. It implements NSView page ranges and page rectangles, then
runs NSPrintOperation with the native dialog. A completion/cancellation result
returns to GPUI while the UI remains responsive. Temporary pages/helper files are
removed after the operation. Windows/Linux report native printing unavailable.
Final automated checks verify the print pipeline using NSPrintSaveJob with an
explicit PDF-only guard and by opening/cancelling a standalone panel.

## Multiple selection and groups (0.8)

`editor::selection` owns normalized selection roots, a primary ID, aggregate
bounds, transforms, group operations and alignment/distribution. App code uses
`select_only`, `set_selection` and `toggle_selected`; it cannot overwrite a public
primary ID and leave other selected objects stale. Ancestor/descendant selection
is normalized so subtrees never move twice. History restores the complete selection.

Moves use original world frames for every gesture update and convert back into
current parent coordinates. Candidate transforms are validated before application.
Group bounds recompute around children while retaining their world positions;
snapping excludes the moving subtree and its containing groups to avoid feedback.
Locked/hidden subtrees are excluded from marquee selection. Inherited locks block
batch mutations; explicit unlock handles selected roots and their locked ancestors.

`Node::is_group` is optional/default-false metadata on an ordinary transparent
Frame. The format remains version 1; old readers can render these as frames.
Group hit-testing selects the container until double-click/Text-tool drill-down.
Grouping requires sibling roots, preserves their order and inserts at the
frontmost selected layer. Ungroup retains child IDs, positions, visibility and
effective opacity. Empty groups are removed when their final child is deleted.

`app::selection` supplies the native inspector; the canvas owns Shift-click,
marquee and gesture dispatch. Text carets/spans stay separate from object selection.
Batch clipboard snapshots include all root subtrees and shared original assets;
pastes assign fresh IDs and select every pasted root in one transaction. Group and
multiple-selection scaling are intentionally unavailable at this milestone.

## Trackpad magnification (0.8.1)

The vendored GPUI macOS view forwards `magnifyWithEvent:` as a first-class
`MagnifyEvent`, rather than synthesizing keyboard modifiers or wheel events.
`app::zoom` tracks incremental gesture scale and the starting pointer anchor.
`layout::zoom_at` derives the page origin from current camera values, preventing
anchor drift when input events arrive before a paint. Limits rebase the gesture
scale so reversing direction responds immediately.

Pinch state is transient and separate from document/history. The native test
invokes the actual view selector with an Objective-C event probe, verifying
platform conversion and dispatch without desktop input injection. It covers
two-finger pan, modifier scroll, phase/limit handling, popover/sidebar isolation
and unchanged document/selection/save state. Physical hardware gestures and
Windows/Linux pinch backends require separate platform validation.

## Document modes and editable Word output (0.9)

`Document::mode` is persisted with a Compose default for old files. Word mode is
a compatibility profile: top-level rich text and images, opaque content, source
proportions and no canvas effects. `word_issues` reports incompatible nodes and
page sizes; `validate_word` rejects exports outside Word mode. General document
validation also rejects unsupported content when the document is marked Word.
Grouping and reparenting are disabled in that profile; candidate clipboard pastes
validate before committing, preventing unsupported subtrees crossing modes.

`app::document_mode` owns the chooser, explicit new-document actions and flow
properties. Compatible mode changes sort blocks into reading order and reflow
through the existing Parley engine with 72-point margins and a 12-point block gap.
Word images fit the text column and retain decoded proportions. The gesture,
inspector and keyboard boundaries prevent free placement and canvas effects.
Mode changes are undoable and new-document actions use existing unsaved prompts.

`export::word` uses pinned docx-rs 0.4.22 without its image feature; the existing
bounded image decoder converts embedded raster files to PNG for inline pictures.
Rich text UTF-8 spans become genuine Word runs/paragraphs, including whitespace,
Unicode, tabs, font names, size, weight/style, color and paragraph settings.
Pages become sections with their individual sizes. Exports atomically replace
their destination only after validation and complete packaging.

DOCX is output only. Word may substitute fonts/reflow; no exact Word pagination
or DOCX import is claimed. The Compose editor currently uses explicit pages and
unlinked text blocks. Raster/print output rejects overflowing Word blocks instead
of silently discarding text; DOCX retains the complete text for Word pagination.

## Creation and direct manipulation (0.10)

`document::presets` builds local editable Blank, Report, Letter and Poster models.
The GPUI creation screen keeps a separate mode/preset/name draft; selecting or
cancelling never mutates the open document. A `Pending::Create` snapshot reuses
Save / Discard / Cancel. Successful creation enters the real editor. Preview
images come from `export::PageRenderer` in a background task, so no parallel
template representation or web UI is required.

Canvas creation uses a transient `Gesture::Create` rectangle. Mouse release
commits one object/history transaction; Escape discards the preview. Bounds are
clamped to the page, reverse drags normalize, and typing keeps the user-sized
height. Selected text borders start ordinary movement after ending text editing.
Word insertion still flows through the restricted layout profile.

`app::context_menu` reuses clipboard and editor actions. `hit_test_context` includes
visible locked objects for inspection/unlocking, while ordinary hits and mutation
guards keep those objects protected. Context menus preserve text ranges, work in
Layers, and snap within the native window. Word Layers show reading order;
Compose Layers retain their back-to-front canvas representation.

## Résumé and photo presets (0.11)

`document::presets::designed` constructs the résumé, portrait résumé, photo cover
and portfolio from regular rich-text, shape and image nodes. The single-column
résumé supports Word flow; the designed photo layouts retain Compose-only
geometry and effects. Two bundled original sample PNGs are embedded as normal
document assets, without a runtime download or additional rendering backend.
The existing image picker replaces them with its standard bounds/style/history
semantics. `Preset::document_name` supplies draft names without constructing or
copying an entire photo document merely to change a creation-screen label.

The eight-preset gallery is a native wrapping grid inside a bounded scroll area.
Its scroll handle resets on New; mode, name and preview stay visible while the
gallery scrolls. Previews still use the shared document exporter in the background.
Verification checks actual Parley text heights, page bounds, original image
proportions, Word résumé export, native creation and image replacement, and exact
photo/document save/reopen. Generated asset provenance is in assets/presets.

## Portable interactions and cloud cutouts (0.12)

`editor::drop_layer` performs arbitrary relative ordering and reparenting in one
snapshot, keeping world coordinates and rejecting locks/cycles. GPUI typed drag
payloads carry page/epoch identity; `app::drag_drop` maps visual row order to model
order and rejects stale drops. Insert/Assets drops create real nodes. OS file drops
use the same path payload on Windows/macOS; bounded background decoding precedes
a coalesced editor batch, so multiple files have one undo step.

`TextStyle::link` is optional/default-none for legacy files. Existing attributed
text edits, slices and styled clipboard operations preserve its ranges. The URL
layer allows http/https/mailto and rejects active/local schemes. `app::links` keeps
the target passage while a native popover edits its address; glyph selection
geometry gates pointer activation. DOCX writes OOXML hyperlinks with relationships.

`image_tools` is platform-independent Rust/reqwest. It uploads one bounded image
to bgclear using explicit free-preview parameters, blocks redirects, bounds reads
and dimensions, checks PNG/alpha/proportions, and gives actionable error states.
No inference runtime/model or GPU processing is introduced. App tasks run off the
UI thread and recheck epoch, asset identity and locks before normal replacement.
Original pixels remain shared by undo. Search cutouts reuse the same processor.

The initial 0.12 setup used GPUI's credential API for Windows Credential Manager /
macOS Keychain. That per-user setup is removed in 0.13 in favor of the shared service
below; provider credentials now live exclusively on the server.
The service's live missing-key endpoint was probed; an authenticated result needs
a user key. Local HTTP tests exercise the exact multipart contract and failure
states without uploading user images. Windows packaging/CI is included; actual
Windows GUI and older-PC performance remain separate validation requirements.

## Shared service (0.13)

`compose-background-service` is a separate Rust/Axum executable, not a desktop
dependency. It owns the provider key, bounded image endpoint, concurrency limits,
per-IP minute/day limits, global daily allowance, typed errors, request IDs and
graceful shutdown. It binds loopback by default; HTTPS is supplied by a controlled
reverse proxy/tunnel. Forwarded IP headers are ignored unless the immediate peer
is explicitly trusted. There is no image persistence or user account/database.

The native app now owns only an optional public service URL. It is injected at
build time with `COMPOSE_BACKGROUND_SERVICE_URL`, with a runtime development
override. HTTPS is required except on loopback; URLs with credentials/query/fragment
are rejected. `image_tools::remove_shared` posts the selected image without an auth
header, validates the returned PNG and maps typed shared-limit/service failures.
Provider-key fields and platform credential loading are removed from the client.

The service reuses the existing strict provider client with explicit free-preview
parameters. Operator secrets come from environment or a mounted secret file;
they never go through the desktop build or service responses/logs. Docker packaging
uses a non-root, read-only runtime and excludes secret files from its build context.
Limits are process-local: initial deployment should remain a single instance.

Tests run an actual local HTTP relay with an injected provider and exercise the
native Remove button, transparent asset, geometry/style, undo/redo and reopening.
This proves transport and integration, not remote-provider mask quality. Public
deployment and authenticated provider validation require the owner's host/key.
