# Third-party notices

Compose's original code is licensed under MIT; see [LICENSE](LICENSE).

- **GPUI 0.2.2** is vendored under Apache-2.0. Its license is preserved at
  [vendor/gpui/LICENSE-APACHE](vendor/gpui/LICENSE-APACHE), and local bridge
  changes are described in [vendor/README.md](vendor/README.md).
- **Lato, Noto Sans, Noto Serif, Noto Sans Mono, Anton and Bebas Neue** retain
  their included font licenses. Sources and individual license files are listed
  in [assets/fonts/SOURCES.md](assets/fonts/SOURCES.md).
- Other Rust dependencies retain their upstream licenses. Exact versions and
  upstream packages are recorded in `Cargo.lock`.

Images found through the in-app web search retain their original ownership and
usage rights. A search result or insertion into a document does not grant a
license to the image. Compose preserves source metadata where available.
# Microsoft Visual C++ runtime (Windows package)

The Windows ZIP includes Microsoft's x64 `vcruntime140.dll` from the Visual
Studio redistributable directory. It is Microsoft software, distributed under
the [Microsoft Visual Studio 2022 runtime license](https://visualstudio.microsoft.com/license-terms/vs2022-cruntime/),
not Compose's MIT license. Keep it beside `compose.exe`.
See the [Visual Studio distributable-code list](https://learn.microsoft.com/en-us/visualstudio/releases/2022/redistribution).
