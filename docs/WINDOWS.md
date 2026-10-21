Compose for Windows
===================

1. Download Compose-Windows.zip from https://compose.waveio.me.
2. Right-click the ZIP and choose Extract All.
3. Open the Compose folder and double-click compose.exe.

This is a portable, unsigned preview build for 64-bit Windows 10/11.
No installer, account or administrator access is required. Windows may show
an unrecognized-app notice because this preview is not code signed.
Keep the package intact. The fonts are embedded in the application.
Graphics compatibility and interactive behavior on physical Windows PCs
still need broader testing.

Getting started
---------------
Choose a blank page or a preset, give it a name, then Create document.
Double-click text to edit. Drag objects to move them. Use the inspector to
change fonts, colors, dimensions and image appearance.

Ctrl+S saves your editable .compose file. Ctrl+Shift+S saves a new copy.
Ctrl+O opens a document. Ctrl+Z undoes; Ctrl+Shift+Z or Ctrl+Y redoes.
Ctrl+C/X/V copies, cuts and pastes. Ctrl+D duplicates selected objects.
Use the Export menu for PNG, .compose copies, or editable DOCX in Word mode.

Documents stay on your computer. Save regularly: desktop crash recovery
is not yet available. Images are embedded in saved .compose documents.
Online image search and optional background removal require the internet;
background removal uploads the selected image to the shared service.

Known preview limits
--------------------
DOCX import, native Windows printing, automatic text flow between pages,
and screen-reader certification are not available yet. Word mode exports
DOCX; Compose canvas mode preserves its editable design in .compose files.

Verify the download
-------------------
Download the adjacent .sha256 file. In PowerShell run:
  Get-FileHash .\Compose-Windows.zip -Algorithm SHA256
Compare the result to the checksum before extracting.

Source and issue reports: https://github.com/Imlokzu/compose
License: MIT. Third-party notices and font licenses are included.
