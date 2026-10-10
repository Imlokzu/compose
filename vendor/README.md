# GPUI bridge

`gpui/` is the crates.io source distribution of GPUI 0.2.2 (Apache-2.0;
upstream license retained). Compose keeps a small documented native bridge:

- `GlyphId` exposes its u32 constructor so Parley's shaped glyphs can be painted.
- `Window::with_element_opacity` is public, with documentation, so image
  primitives in a custom canvas inherit opacity correctly.

- `DispatchEventResult` is public, with documentation, enabling the native smoke
  test to use the already-public `dispatch_event` method.

No layout, font matching, or renderer implementation is forked. Compare against
crates.io 0.2.2 before upgrading; remove these patches if upstream exposes the APIs.

Compose 0.8.1 additionally introduces `MagnifyEvent` / `PlatformInput::Magnify`.
The macOS view registers `magnifyWithEvent:` and converts AppKit magnification,
phase, position and modifiers into that event. Window dispatch handles it as a
mouse-positioned input event. The application subscribes through the existing
paint-phase `Window::on_mouse_event` API; no renderer/layout code is changed.
