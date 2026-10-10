# Compose feature audit — 2026-10-10

This covers Compose 0.8's working native editor. Rust, GPUI, Parley and local `.compose`
files remain the constraints. Cloud, accounts, AI and presentations stay outside
this scope.

## Delivered in this iteration

- 0.7: editable `.compose` export, current-page PNG at three resolutions and
  native macOS printing with PDF output.
- 0.8: Shift-click/marquee selection, batch movement/nudging, clipboard,
  duplicate/delete/ordering, six alignment controls, equal spacing, grouping,
  group drill-down and automatic bounds. Existing text passage formatting and
  image assets remain intact; group save/reopen and unchanged PNG pixels are checked.

- Object copy/cut/paste: text boxes, shapes, images and complete nested frames.
  Fresh IDs, original referenced image bytes, incremental paste offsets and one
  undo transaction. Cut respects inherited locks. Object data survives document
  switches within the same running app; other applications get readable text.
- Formatted passage clipboard: copied text retains fonts, sizes, colors and
  inline styles. Fields receive plain text. External text pasted onto the canvas
  becomes an editable text box.
- Replace image: reuses the local/Commons picker and retains layer ID/name,
  hierarchy, position, width, opacity, radius, stroke and shadow. Scale adjusts
  height to the new source ratio; Stretch retains both dimensions. Attribution
  follows the replacement. Cancellation leaves the original untouched.

Implementation: `crates/editor/src/clipboard.rs`, `crates/app/src/clipboard.rs`,
`crates/text/src/lib.rs`, `crates/app/src/image_picker.rs`.

## Remaining gaps, ranked for the next milestones

| Priority | Feature | Repository evidence | Benefit and implementation risk |
| --- | --- | --- | --- |
| 1 | Text flow, lists and tables | `RichText` has inline spans and one alignment per box; `Workspace::measure` grows independent boxes | Support longer reports. Paragraph structure, overflow, carets and reflow must agree before automatic pagination. |
| 2 | Crash recovery and autosave | `Workspace::save/save_to/request` implement explicit saves and unsaved prompts; no recovery store | Prevent work loss without silently overwriting explicitly saved files. |
| 3 | Find/replace and document navigation | No find action or match state in `Workspace` | Unicode-safe search across text boxes/pages, reveal selection and atomic replacement. |
| 4 | Image crop and external image paste | `ImageSizing` offers Scale/Stretch; clipboard consumes text or local object tokens | Crop coordinates must preserve originals; OS-image decoding must run off the UI thread. |
| 5 | Selection/group scaling and batch style controls | `Workspace::resizable_selection` exposes handles only for individual non-group nodes; `selection_inspector` is about organization | Define text-size versus reflow behavior, proportions, child transforms and opacity semantics before adding handles/fields. Grouping across different parents also remains unavailable. |
| 6 | Vector PDF and additional export formats | `export::PageRenderer` generates raster pages; native Print uses 300 DPI PNGs | Vector glyph/font embedding and DOCX/HTML fidelity need their own output backends and visual tests. |
| 7 | Linked components, rotation and vector shapes | Component insertion produces independent layouts; no rotation/vector geometry model | Follow everyday writing, recovery and layout features. |

Platform validation remains a release gate: macOS is locally exercised; Windows,
Linux, accessibility and system-font portability have not been certified.

## Delegation evidence

Lightweight delegation was attempted but returned no successful output. The
connector lists DeepSeek V4 Flash, not V4.1 Flash. Its Pioneer host returns HTTP
410; the supplied Fastino endpoint rejects the retired stored key with HTTP 401.
Native analyst/executor roles fail HTTP 422 because their fixed `gpt-5.4-mini`
model is unsupported; the explicit lightweight override did not change it. A
bounded Codex CLI reviewer using `gpt-6-luna` also failed authentication.

No global configuration or credentials were changed. The main agent completed
this audit and implementation. No independent review verdict is claimed.

The 0.8 sidecar audit also failed: the installed `explore` role's fixed
`gpt-5.3-codex-spark` model returned HTTP 422. The agent was closed and the main
agent completed the review and regression checks locally.
