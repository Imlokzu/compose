#!/usr/bin/env python3
"""Stage a verified Windows package for static Cloudflare hosting."""
import hashlib
import shutil
import struct
import sys
import tempfile
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
    required = {"Compose/compose.exe", "Compose/vcruntime140.dll", "Compose/LICENSE", "Compose/START-HERE.txt", "Compose/GPUI-LICENSE.txt"}
    if not required.issubset(archive.namelist()):
        raise SystemExit("Windows package is missing its executable or required documentation.")
    if archive.testzip() is not None:
        raise SystemExit("Corrupt ZIP; refusing to publish.")
    executable = archive.read("Compose/compose.exe")
    if len(executable) < 64 or executable[:2] != b"MZ":
        raise SystemExit("Package does not contain a Windows executable.")
    pe_offset = struct.unpack_from("<I", executable, 0x3C)[0]
    if (pe_offset + 6 > len(executable)
            or executable[pe_offset:pe_offset + 4] != b"PE\0\0"
            or struct.unpack_from("<H", executable, pe_offset + 4)[0] != 0x8664):
        raise SystemExit("Package must contain an x64 Windows PE executable.")
if source.stat().st_size > 25 * 1024 * 1024:
    raise SystemExit("ZIP exceeds the Cloudflare static-asset limit. Use object storage for this release.")
destination = Path(__file__).resolve().parent / "public" / "downloads"
destination.mkdir(parents=True, exist_ok=True)
# Stage outside public assets, on the same filesystem as the final files.
with tempfile.TemporaryDirectory(prefix=".release-stage-", dir=destination.parent.parent) as tmp:
    staged_zip = Path(tmp) / "Compose-Windows.zip"
    staged_checksum = Path(tmp) / "Compose-Windows.zip.sha256"
    shutil.copyfile(source, staged_zip)
    staged_checksum.write_text(f"{digest}  Compose-Windows.zip\n")
    staged_zip.replace(destination / staged_zip.name)
    staged_checksum.replace(destination / staged_checksum.name)
print(f"Staged {source.stat().st_size:,} bytes; SHA-256 {digest}")
