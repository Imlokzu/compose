#!/usr/bin/env python3
"""Stage a verified Windows package for static Cloudflare hosting."""
import hashlib
import shutil
import sys
import zipfile
from pathlib import Path

source = Path(sys.argv[1]) if len(sys.argv) == 2 else Path("../dist/Compose-Windows.zip")
checksum_file = source.with_suffix(".zip.sha256")
if not source.is_file() or not checksum_file.is_file():
    raise SystemExit("Provide a Windows ZIP and the matching .zip.sha256 from the same CI artifact.")
digest = hashlib.sha256(source.read_bytes()).hexdigest()
if checksum_file.read_text(encoding="utf-8-sig").split()[0].lower() != digest:
    raise SystemExit("Checksum mismatch; refusing to publish this package.")
with zipfile.ZipFile(source) as archive:
    required = {"Compose/compose.exe", "Compose/LICENSE", "Compose/START-HERE.txt", "Compose/GPUI-LICENSE.txt"}
    if not required.issubset(archive.namelist()):
        raise SystemExit("Windows package is missing its executable or required documentation.")
    if archive.testzip() is not None:
        raise SystemExit("Corrupt ZIP; refusing to publish.")
    if archive.read("Compose/compose.exe")[:2] != b"MZ":
        raise SystemExit("Package does not contain a Windows executable.")
if source.stat().st_size > 25 * 1024 * 1024:
    raise SystemExit("ZIP exceeds the Cloudflare static-asset limit. Use object storage for this release.")
destination = Path(__file__).resolve().parent / "public" / "downloads"
destination.mkdir(parents=True, exist_ok=True)
shutil.copyfile(source, destination / "Compose-Windows.zip")
(destination / "Compose-Windows.zip.sha256").write_text(f"{digest}  Compose-Windows.zip\n")
print(f"Staged {source.stat().st_size:,} bytes; SHA-256 {digest}")
