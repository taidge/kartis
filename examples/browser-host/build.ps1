param()
$ErrorActionPreference = 'Stop'
Push-Location $PSScriptRoot
try {
    $taskMetadataText = & cargo metadata --no-deps --format-version 1
    if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed' }
    $taskMetadata = $taskMetadataText | ConvertFrom-Json
    & cargo build --target wasm32-unknown-unknown --release
    if ($LASTEXITCODE -ne 0) { throw 'browser-host Cargo build failed' }
    $taskArtifact = Join-Path $taskMetadata.target_directory 'wasm32-unknown-unknown/release/kartis_host_fixture.wasm'
    $taskPackage = Join-Path $PSScriptRoot 'pkg'
    & wasm-bindgen $taskArtifact --target web --out-dir $taskPackage --out-name host
    if ($LASTEXITCODE -ne 0) { throw 'browser-host wasm-bindgen failed' }
    Write-Output "Browser host artifacts: $taskPackage"
} finally { Pop-Location }
