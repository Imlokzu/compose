$ErrorActionPreference = "Stop"
$composePreviousServiceUrl = $env:COMPOSE_BACKGROUND_SERVICE_URL
$composeStage = Join-Path ([System.IO.Path]::GetTempPath()) ("compose-package-" + [guid]::NewGuid())
Push-Location (Join-Path $PSScriptRoot "..")
try {
    if ([string]::IsNullOrWhiteSpace($env:COMPOSE_BACKGROUND_SERVICE_URL)) {
        $env:COMPOSE_BACKGROUND_SERVICE_URL = 'https://compose-background.onrender.com/v1/background/remove'
    }
    cargo build -p compose --release --locked
    if ($LASTEXITCODE -ne 0) { throw "Windows build failed" }
    $composeOutput = Join-Path $composeStage "Compose"
    New-Item -ItemType Directory -Force $composeOutput | Out-Null
    New-Item -ItemType Directory -Force dist | Out-Null
    Copy-Item target/release/compose.exe $composeOutput
    # Rust's MSVC target imports this redistributable DLL. Include it app-locally
    # so a clean Windows PC does not need a separate administrator installation.
    $composeVsWhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
    $composeVsRoot = & $composeVsWhere -latest -products '*' -property installationPath
    if ($LASTEXITCODE -ne 0 -or -not $composeVsRoot) { throw 'Visual Studio installation not found' }
    $composeRuntime = Get-ChildItem "$composeVsRoot/VC/Redist/MSVC/*/x64/Microsoft.VC*.CRT/vcruntime140.dll" |
        Sort-Object FullName -Descending | Select-Object -First 1
    if (-not $composeRuntime) { throw 'The x64 Visual C++ redistributable runtime was not found' }
    Copy-Item $composeRuntime.FullName (Join-Path $composeOutput 'vcruntime140.dll')
    Copy-Item LICENSE $composeOutput
    Copy-Item THIRD_PARTY_NOTICES.md $composeOutput
    Copy-Item docs/WINDOWS.md (Join-Path $composeOutput "START-HERE.txt")
    Copy-Item vendor/gpui/LICENSE-APACHE (Join-Path $composeOutput "GPUI-LICENSE.txt")
    Copy-Item assets/fonts/*LICENSE*.txt $composeOutput
    Copy-Item assets/fonts/*OFL*.txt $composeOutput
    # Include a single enclosing folder and never include stale build output.
    Compress-Archive -Path $composeOutput -DestinationPath dist/Compose-Windows.zip -Force
    $composeHash = (Get-FileHash dist/Compose-Windows.zip -Algorithm SHA256).Hash.ToLowerInvariant()
    "$composeHash  Compose-Windows.zip" | Set-Content -Encoding ascii dist/Compose-Windows.zip.sha256
    Write-Output "Built dist/Compose-Windows.zip"
} finally {
    if (Test-Path $composeStage) { Remove-Item -Recurse -Force $composeStage }
    $env:COMPOSE_BACKGROUND_SERVICE_URL = $composePreviousServiceUrl
    Pop-Location
}
