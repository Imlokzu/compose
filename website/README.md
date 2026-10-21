# Compose download site

The public Windows download site is deployed to Cloudflare Workers Static Assets
at **https://compose.waveio.me**. It runs entirely on Cloudflare; the developer's
computer does not need to stay online. The native editor runs on the user's PC.

## Develop and check

```sh
cd website
npm ci
npm run dev
npm run check
```

The website uses plain HTML/CSS, local bundled fonts, real editor screenshots
and rendered preset previews. It needs no frontend framework, browser JavaScript,
analytics, cookies or application secrets. Font licenses are in `public/assets`.
Preset image provenance is recorded in `../assets/presets/SOURCES.md`.

## Publish a Windows build

1. Wait for the native desktop workflow's Windows job to pass for the intended
   source commit. The job builds and tests Rust, packages the release executable,
   extracts the ZIP and runs the extracted executable's document-engine smoke test.
2. Download the `Compose-Windows` artifact with `gh run download RUN_ID -n
   Compose-Windows -D ../dist/windows-release`. Do not mix files from different runs.
3. Stage and verify the archive:

   ```sh
   python3 prepare-release.py ../dist/windows-release/Compose-Windows.zip
   ```

4. Update the public version text and release notes if the version changed.
5. Use a Cloudflare login or environment-provided API credentials, then run
   `npm run deploy`. The custom-domain route in `wrangler.jsonc` configures
   `compose.waveio.me`. Never put credentials in this repository or public assets.
6. Download the production ZIP, compare its SHA-256 to the CI artifact, inspect
   the HTTP headers and verify the home page, Windows guide and 404 page.

Binary downloads are intentionally ignored by Git. They are included in the
Cloudflare asset deployment after staging and can also be attached to a GitHub
prerelease for archival. `prepare-release.py` refuses corrupt, incomplete,
mismatched or oversized archives. Static assets have a 25 MiB per-file limit;
larger packages need a separately configured object store.

Preview status is deliberate: the Windows ZIP is unsigned; automated engine
checks do not certify GPU compatibility, full interactive behavior or accessibility
on physical Windows PCs. Keep this distinction in the public release notes.
