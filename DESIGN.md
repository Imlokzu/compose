# Design

## Source of truth
- Status: Active. Refreshed 2026-10-11.
- Surface: Compose native document editor.
- Evidence: user brief, brand/reference-ui.jpg, brand/logo-original.jpg.

## Brand
Calm, precise, native. Two offset pages as the mark. Avoid ribbons, dashboard cards, gratuitous gradients and large navigation.

## Product goals
Write like a document. Design like a canvas. Real editing, placement and local save/reopen. Word-compatible documents export editable DOCX. Exclude cloud, accounts, AI, presentations and DOCX import. Success means a user can create and reopen a designed page with text and an image.

## Android editor (0.15)
- Keep the shared Rust document/history/Parley/export engines. Kotlin/Jetpack
  Compose supplies Android controls, keyboard, file/photo pickers and touch input.
- Phones use a focused canvas, compact top bar and 48dp bottom controls for Text,
  Shapes, Image, Layers and Properties. Panels are bounded native bottom sheets.
  Tablets show Layers beside the canvas. Preserve warm neutrals and blue selection.
- Native text editing uses Android selection, keyboard/IME and clipboard. Its
  formatting controls edit highlighted UTF-16 ranges through the Rust rich-text
  model. Document previews and PNG exports use the shared Parley glyph renderer.
- Pinch zoom stays under the fingers; two fingers pan. Selected objects drag and
  resize with generous touch handles. Movement is one history step per gesture.
- Start screen offers the real editable presets and both document modes. Local
  recovery protects work across background/process recreation. Save/open use
  Android's document picker; .compose files exchange with the desktop app.
- Validate phone/tablet, keyboard, Unicode range formatting, drag/resize, images,
  history, file roundtrip, export and lifecycle on emulators. Keep unavailable
  desktop-only features out of mobile controls and report platform limits honestly.

## Personas and jobs
Students and people making homework, reports, worksheets, posters and covers on desktop.

## Information architecture
Thin top toolbar; current page's layers and small insert area on left; A4 on a neutral canvas; contextual inspector on right. Pages selector belongs above layers. No file explorer.

## Design principles
Text and object interactions share one source of truth. Prefer direct manipulation; always show state and actionable errors. Only expose working features.

## Visual language
Warm grey canvas #ECEDEB, white panels, page #FFFEFB, ink #232624, muted #777C76, dividers #E3E5E0, selection blue #2878ED. UI system sans 12–13px, headings medium. 4px spacing rhythm; toolbar 48px, sidebars 220/272px. Controls 28–32px, radius 4–6px. Hairline borders and subtle page shadow. Consistent line icons, no emoji. No nonessential motion.

## Components
Topbar tool buttons, page/layer rows, inspector fields, compact text formatting bar, object handles, snap guides, status strip. Shared colors and sizing in app/panels.rs; extract a UI crate when reused.

## Accessibility
Keyboard commands and visible focus. Labels for tools and fields. Readable contrast. GPUI accessibility coverage must be tested before claiming screen-reader support. Respect native input/IME.

## Responsive behavior
Desktop minimum 1000×680; default 1360×900. Flexible center viewport, fixed sidebars, pan/zoom when page exceeds viewport.

## Interaction states
Empty page invites Text tool or double-click. Saving and errors appear in status. Disabled actions have muted styling. All work local and offline. Select outlines and eight resize handles; guides only while snapping.

## Content voice
Short, plain labels: Layers, Insert, Text, Image, Position, Size, Save. No development details in primary flows.

## Implementation constraints
Rust + GPUI; Parley owns document text layout. Coordinates in points, zoom only changes rendering. Verify build, engine tests, file round-trip and native launch. Never claim visual QA from compilation alone.

## Open questions
- Cross-platform packaging and accessibility need platform-specific validation beyond this macOS environment.
- Rotation, pagination across linked text frames and additional export formats are later milestones.

## Shape library (0.14)
- Reuse the compact native popover and toolbar controls. Shapes opens a two-column
  picker with labeled line icons: Rectangle, Circle, Oval, Triangle, Diamond,
  Pentagon, Hexagon, Star, Arrow and Line. Choosing a type arms click/drag creation.
  The same picker changes an existing shape from its inspector in one undo step.
- Circle keeps equal dimensions through creation, handles and numeric fields.
  Oval resizes independently; Shift constrains other figures while drawing.
  Line is a real two-endpoint stroke, with thickness and color controls.
- Preserve fill, opacity, rounded polygon corners, outlines, shadows, layers,
  dragging, duplicates and history. Show corner controls only for figures that
  actually support rounding. Word mode retains its canvas-shape restriction.
- Native rendering, PNG/print and hit-testing share document-owned geometry.
  Select the visible shape, allowing clicks through empty oval/polygon corners.
  Older shape nodes without a subtype reopen as rectangles.
- Validate native picker gestures, both line directions, circle proportions,
  exact save/reopen and exports, including styled curved and concave figures.

## Refinement: image sizing and partial text
- Scale is the default for images. Both side and corner handles preserve source proportions; dimension fields update the paired value. Bigger/Smaller uniformly scale around the center. Original image bytes are retained.
- Stretch is an explicit alternative with independent width and height. Mode is stored in the document and undoable. Returning to Scale restores the source ratio.
- Corners: exact numeric radius, Square/Soft/Round presets, and an inset drag handle. Clamp to half the shortest side for rendering. Images and shapes use the same controls.
- Text tool edits existing text. Double-click selects a word; dragging selects a range. Formatting fields preserve that selection, show Mixed for multiple values, and modify only the selected range.
- Floating text controls sit near the selection, stay inside the viewport and block pointer fall-through to the canvas. The inspector identifies whether it edits selected text or the text box.
- Inspector controls share a 4px spacing rhythm and full-field click targets; selected icons use the same blue as selection outlines.
- Verification: non-square image at every resize handle, proportionate numeric edits and scaling, deliberate stretching, radius edits, mouse-selected text with untouched surrounding styles, history, roundtrip, and native screenshots.

## Product polish goal (0.3)
- Six bundled type families: humanist sans, neutral sans, editorial serif, mono,
  condensed display and poster display. Preview names in their actual family.
  Offer only real bundled weights; retain per-range formatting and Mixed states.
- Colors: continuous saturation/value and hue controls, hex + alpha, curated
  ramps, and colors already used in the document. Color drags are one undo step.
- Appearance: optional stroke and soft shadow for visual objects; never bake
  styles into image pixels. Color, stroke and shadow survive native round-trips.
- Assets tab contains editable Title, Callout, Quote, Stat, Card and Divider blocks.
  Each is built from existing frame/text/shape nodes, with no synchronized-instance
  claim. Insertion takes one click and restores the Layers view.
- Layer visibility/locking controls, clear selection states and small icons.
  Preview hides editor chrome and prevents document mutations until closed.
- Editable document title and page sizes/orientation. Panels stay compact;
  popovers occlude the canvas and dismiss with Escape or outside click.
- Use native focus/text-input plumbing for picker fields. Persist document state,
  not popover or camera state. Validate all changes in the running native app.

## Image insertion (0.4)
- Every Image entry opens one native picker: My computer / Search online.
- Local mode uses the OS file dialog. Search mode explicitly names Wikimedia
  Commons and shows a query, suggestion chips, thumbnail grid and selected-image
  details (title, dimensions, creator, license, source link).
- Enter submits the query; typing alone never sends requests. Search/downloads
  run off the UI thread. Loading, no results, failed previews, retry and insertion
  errors have visible states. Older responses cannot replace a newer query.
- Insert downloads and embeds the original supported raster file; it remains
  available offline, scales proportionately and has normal history/layer behavior.
- Source and license metadata is saved with the image and available in its inspector.
  Opening the original source is explicit; inserting does not launch a browser.
- Closing the picker or opening another document cancels pending UI effects.
  Network/decoder byte, pixel and timeout limits protect responsiveness.

## Everyday editing (0.5)
- Clipboard follows context: passages retain inline formatting, fields receive
  plain text, selected objects copy complete subtrees with original image assets.
  Pasted roots are visible/unlocked, with fresh IDs and incremental offsets. Cut
  respects inherited locks. Object data survives document switches in the running
  instance; other apps/instances receive the readable text fallback.
- Image inspector: compact Replace image action reuses the local/online modal,
  with an explicit replacement title and action. Preserve position, width, name,
  identity, nesting and appearance. Scale follows the new ratio; Stretch preserves
  both dimensions. Explain the behavior beside the action before replacement.
  Cancellation leaves the image untouched; success is one undo step.
- Reuse existing buttons, labels, sections, focus and error patterns. Fields in
  modal pickers retain clipboard support; preview prevents document mutation.
  Verify content/styles, locks, cancellation, history and native save/reopen.
  Remaining product scope is ranked in `docs/FEATURE_GAPS.md`.

## Selection typography menu (0.5.1)
- Center an 84px two-row toolbar over the visible bounds of the highlighted
  passage, including multiple lines. Clamp within the canvas, place below when
  the upper edge has insufficient space, and hide when selection is offscreen.
  Use the current camera transform so pan/zoom and window resizing stay aligned.
- First row: searchable font, actual available weight, exact size and size presets.
  Second row: text styles, bold/italic/underline, text color and clear formatting.
  Keep 30px controls, subtle separators and compact spacing. Show Mixed states.
- Reuse the six bundled font previews and continuous color picker. Menus opened
  from selection stay near the toolbar and fit within the canvas; inspector menus
  keep their existing placement. Fonts search receives focus; Enter applies its
  first matching font. Menu typing/navigation preserves document text selection.
- Seven inline typography presets: Title, Heading, Body, Caption, Subheading,
  Quote and Code. Quote uses italic Noto Serif; Code uses Noto Sans Mono. Other
  presets retain the selected font. Color and underline survive preset changes.
  Clear formatting restores defaults only on the highlighted passage.
- Rejected field values stay focused on Enter so correction affects the field,
  preserving highlighted document text. Escape cancels the field edit.
- All controls preserve the passage and surrounding styles. One operation is one
  undo step. Verify real pointer actions, search input, camera/edge placement,
  partial styles and save/reopen in both default and minimum native windows.

## Web image search (0.6)
- Search online defaults to full web image results from Bing Images. Commons is
  a clearly separate optional source, never presented as general web search.
- Native source buttons, query, thumbnail grid, title and originating website.
  Preserve source links on inserted images; do not invent creator/license data
  when the web result has none. Show usage-rights metadata only when provided.
- Downloads embed originals and use normal geometry/history. Failed originals
  can be retried; offer explicit Insert preview when its cached thumbnail is
  available. Label lower resolution in status and saved source details. Offer
  Open search in browser as an explicit action on search failure.
- Parse public result metadata in Rust; render controls natively in GPUI. No
  webview, API secret, CAPTCHA workaround or document upload. Preserve bounded
  background reads/decoding, public-address validation and cancellation.

## Export and print (0.7)
- One compact Export popup in the top bar and in page preview; no separate export
  screen. Keep Save visible and provide native File-menu actions.
- Compose file exports every editable page/asset as a copy. PNG exports the current
  page, with 1×, 2× and 300 DPI choices and the resulting dimensions shown upfront.
- Print opens the system print dialog for all pages. macOS exposes printer, copies,
  page range and Save as PDF. Cancelling does not alter the working document.
- Output omits selections, carets, guides and page labels, and uses page coordinates
  so zoom/pan/window size cannot change the exported layout.
- Show preparing/exporting/completion/errors in the existing status bar. Perform
  rendering and encoding off the UI thread; keep native-file and PNG writes atomic.

## Selection and organization (0.8)
- Shift-click adds/removes objects in both canvas and Layers. An empty-space drag
  draws a restrained blue marquee; Shift-drag adds to the selection.
- Multiple objects have individual subtle outlines and one shared blue movement
  box. The inspector shows their count/bounds, alignment, spacing and group actions.
  Never show resize handles for unimplemented selection/group scaling.
- Groups keep the page's appearance and act as one object. Double-click drills
  into contents; Escape returns to the parent group. Text still selects/edits
  passages, with the existing contextual formatting toolbar.
- Keep native Edit menu shortcuts (⌘G / ⇧⌘G), clear disabled states for locks and
  non-sibling grouping, and one undo transaction per gesture or batch command.

## Document modes (0.9)
- Keep the mode visible as Canvas / Word beside zoom and export. The chooser
  explains the two profiles, switching behavior and new-document shortcuts.
- Word mode uses a restrained single-column text/image flow. Its inspector shows
  typography, text color, proportionate image width and reading-order actions.
  Canvas tools/effects are disabled; DOCX export is enabled only for this profile.
- Compose remains the default for existing files and keeps the full design editor.
  DOCX export is disabled. Reject incompatible switching without removing content.
- Both modes use `.compose` saves. Mode changes are undoable; new documents use
  unsaved-change prompts. Show overflow warnings and avoid clipped PNG/print output.

## Creating and manipulating documents (0.10)
- Launch without a file opens a native creation screen. Choose Blank, Report,
  Letter or Poster and Compose / Word-compatible before creating. File → New
  returns to this screen; opening a saved file goes directly to its editor.
- Preset previews render the actual editable document model. All preset text,
  shapes and layers remain editable; no account, template download or web UI.
- Explain Word's reading-order layout and unsupported canvas effects before
  creation. Poster requires Compose. Never silently strip content for DOCX;
  incompatible documents keep their content and show an actionable explanation.
- Choosing a preset or opening creation never changes the working document.
  Creating a replacement respects Save / Discard / Cancel. Cancel returns to
  the working document without losing changes.
- In Compose, Text / Shape / Frame drag out their bounds before insertion.
  A click creates one practical default box; Escape cancels a drawn preview.
  Reverse drags work, page edges constrain creation, and typing preserves the
  user's box height while expanding when text needs more room.
- Single-click selects and drags objects; double-click edits text. Drag the
  selected text border to move it even while editing. Do not spawn objects on
  right-click. Word blocks retain reading-order layout and explain this limit.
- Right-click on canvas objects or Layers opens a compact, edge-clamped menu:
  text editing / move, clipboard, duplicate, rename, ordering, lock and delete.
  Preserve highlighted text for its clipboard menu; honor locks and history.

## Résumé and photo presets (0.11)
- Add a clean single-column résumé that supports both profiles, plus Compose
  layouts for a portrait résumé, large-photo cover and project portfolio.
- Real rich text, original embedded photos, separate shapes and descriptive
  layer names. Photos use proportionate Scale, with editable rounded corners.
  Replace image uses the existing local/web picker and normal undo history.
- Keep an eight-layout native gallery in a bounded scroll area. New résumé/photo
  choices appear in the first row; the original report, letter and poster remain
  available. The preview always renders the document source, never a mockup.
- Explain Compose-only layouts and photo replacement near the selected preview.
  Preserve existing Word/Compose mode rules; never strip columns/effects for DOCX.
- Validate actual text heights, picture proportions, persistence, Word résumé
  export, original preset access after scrolling and the minimum native window.

## Drag, links and portable image cutouts (0.12)
- Drag Layers rows above/below another row to reorder. Show a blue insertion
  edge and a compact name preview. Canvas order is reversed; Word order follows
  reading order. Preserve world positions, locks, nesting validity and one undo step.
- Drag Text/Shape/Frame from Insert or a component from Assets onto the page.
  Image drops open the picker at the chosen position. Native file-manager image
  drops decode in the background; importing several is a single undo action.
- Add links to highlighted text with Ctrl/⌘K, a small toolbar icon, inspector or
  context menu. Keep selection through editing; Apply, Remove and Open work.
  Ctrl/⌘-click follows links during editing; Preview uses a normal click.
  Store links in rich text and preserve clickable DOCX output.
- User chose cloud background removal for Windows and slower PCs. Use bgclear's
  documented free preview size only (up to 800 px). No local model/GPU inference.
  Show the upload/resolution note before the operation. The initial per-user
  credential setup was replaced by the shared service in 0.13.
- Image inspector: Remove background and About. Search: Without
  background downloads then processes the selected original in the background.
  Preserve originals/placement/style and undo; failures never replace the image.
- Validate shared engine/API tests and actual native gestures. Windows build
  workflow is separate from Windows execution claims; hardware performance needs
  actual old-PC measurement before promising a minimum CPU/RAM experience.

## Shared Compose background service (0.13)
- Users do not enter API keys or create accounts. Remove background / Without
  background use one public Compose endpoint; only the owner configures the
  provider key on the server. The app contains no provider credential handling.
- Keep one compact About popup explaining upload, preview resolution and shared
  limits. A build without a connected service says so without offering fake
  results or returning to the old per-user API-key setup.
- The Rust service accepts one image, performs a free-preview provider request,
  and returns a validated transparent PNG. No images/accounts/database are stored.
- Preserve direct interaction, errors, undo and original geometry. Limits and
  service failures are understandable; a failed request keeps the original image.
- Validate a real local relay and actual native button using a controlled provider
  fixture. Distinguish this from live-provider verification and public deployment.
