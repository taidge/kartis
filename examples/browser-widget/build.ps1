param()
$ErrorActionPreference = 'Stop'
Push-Location $PSScriptRoot
try {
    $taskMetadataText = & cargo metadata --no-deps --format-version 1
    if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed' }
    $taskMetadata = $taskMetadataText | ConvertFrom-Json
    & cargo build --target wasm32-unknown-unknown --release
    if ($LASTEXITCODE -ne 0) { throw 'browser-widget Cargo build failed' }
    $taskArtifact = Join-Path $taskMetadata.target_directory 'wasm32-unknown-unknown/release/kartis_widget_fixture.wasm'
    $taskPackage = Join-Path $PSScriptRoot 'pkg'
    & wasm-bindgen $taskArtifact --target web --out-dir $taskPackage --out-name widget
    if ($LASTEXITCODE -ne 0) { throw 'browser-widget wasm-bindgen failed' }
    Write-Output "Browser Widget artifacts: $taskPackage"
} finally {
    Pop-Location
}
