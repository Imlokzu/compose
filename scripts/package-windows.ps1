$ErrorActionPreference = "Stop"
Push-Location (Join-Path $PSScriptRoot "..")
try {
    cargo build -p compose --release --locked
    if ($LASTEXITCODE -ne 0) { throw "Windows build failed" }
    $composeOutput = Join-Path (Get-Location) "dist/Compose"
    New-Item -ItemType Directory -Force $composeOutput | Out-Null
    Copy-Item target/release/compose.exe $composeOutput
    Copy-Item LICENSE $composeOutput
    Copy-Item vendor/gpui/LICENSE-APACHE (Join-Path $composeOutput "GPUI-LICENSE.txt")
    Copy-Item assets/fonts/*LICENSE*.txt $composeOutput
    Copy-Item assets/fonts/*OFL*.txt $composeOutput
    Compress-Archive -Path "$composeOutput/*" -DestinationPath dist/Compose-Windows.zip -Force
    Write-Output "Built dist/Compose-Windows.zip"
} finally { Pop-Location }
