param([switch]$Debug)
$ErrorActionPreference = 'Stop'
Push-Location $PSScriptRoot
try {
    $metadataJson = & cargo metadata --no-deps --format-version 1
    if ($LASTEXITCODE -ne 0) { throw 'Cargo metadata failed' }
    $metadata = $metadataJson | ConvertFrom-Json
    $targetDirectory = $metadata.target_directory
    $buildArguments = @('build', '--locked', '--target', 'wasm32-unknown-unknown')
    $buildProfile = 'debug'
    if (-not $Debug) { $buildArguments += '--release'; $buildProfile = 'release' }
    & cargo @buildArguments
    if ($LASTEXITCODE -ne 0) { throw 'Browser WASM build failed' }
    if (-not (Get-Command wasm-bindgen -ErrorAction SilentlyContinue)) {
        throw 'wasm-bindgen CLI 0.2.130 is required. Install with: cargo install wasm-bindgen-cli --version 0.2.130 --locked'
    }
    $wasm = Join-Path $targetDirectory "wasm32-unknown-unknown/$buildProfile/kartis_canvas_demo.wasm"
    & wasm-bindgen $wasm --target web --out-dir (Join-Path $PSScriptRoot 'pkg') --out-name kartis_canvas_demo
    if ($LASTEXITCODE -ne 0) { throw 'wasm-bindgen generation failed' }
    Write-Host "Browser assets ready in $(Join-Path $PSScriptRoot 'pkg')"
} finally {
    Pop-Location
}

