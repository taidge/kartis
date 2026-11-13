param(
    [switch]$SkipWasm,
    [switch]$BuildBrowser,
    [switch]$RunNative,
    [switch]$RunMemory
)
$ErrorActionPreference = 'Stop'
function Invoke-Checked {
    param([string]$Program, [string[]]$Arguments)
    & $Program @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Program failed with exit code $LASTEXITCODE" }
}
Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    $metadataJson = & cargo metadata --no-deps --format-version 1
    if ($LASTEXITCODE -ne 0) { throw 'Cargo metadata failed' }
    $metadata = $metadataJson | ConvertFrom-Json
    Write-Host "Workspace artifacts: $($metadata.target_directory)"
    Invoke-Checked -Program cargo -Arguments @('fmt', '--package', 'kartis', '--check')
    Invoke-Checked -Program cargo -Arguments @('check', '--locked', '--no-default-features', '--all-targets')
    Invoke-Checked -Program cargo -Arguments @('test', '--locked', '--no-default-features', '--lib', '--tests')
    Invoke-Checked -Program cargo -Arguments @('test', '--locked', '--all-features', '--all-targets')
    Invoke-Checked -Program cargo -Arguments @('test', '--locked', '--all-features', '--doc')
    Invoke-Checked -Program cargo -Arguments @('clippy', '--locked', '--all-features', '--all-targets', '--', '-D', 'warnings')
    if (-not $SkipWasm) {
        Invoke-Checked -Program cargo -Arguments @('check', '--locked', '--target', 'wasm32-unknown-unknown', '--all-features', '--lib')
    }
    foreach ($example in @('browser-canvas', 'browser-widget', 'browser-host', 'browser-raster')) {
        Push-Location (Join-Path $PWD "examples/$example")
        try {
            $exampleMetadataJson = & cargo metadata --no-deps --format-version 1
            if ($LASTEXITCODE -ne 0) { throw 'Browser example metadata failed' }
            $exampleMetadata = $exampleMetadataJson | ConvertFrom-Json
            Write-Host "$example artifacts: $($exampleMetadata.target_directory)"
            $packageName = $exampleMetadata.packages[0].name
            Invoke-Checked -Program cargo -Arguments @('fmt', '--package', $packageName, '--check')
            if (-not $SkipWasm) {
                Invoke-Checked -Program cargo -Arguments @('clippy', '--locked', '--target', 'wasm32-unknown-unknown', '--', '-D', 'warnings')
            }
            if ($BuildBrowser) { & './build.ps1' }
        } finally { Pop-Location }
    }
    foreach ($example in @('native-raster')) {
        Push-Location (Join-Path $PWD "examples/$example")
        try {
            $nativeMetadataJson = & cargo metadata --no-deps --format-version 1
            if ($LASTEXITCODE -ne 0) { throw 'Native example metadata failed' }
            $nativeMetadata = $nativeMetadataJson | ConvertFrom-Json
            Write-Host "$example artifacts: $($nativeMetadata.target_directory)"
            Invoke-Checked -Program cargo -Arguments @('fmt', '--check')
            Invoke-Checked -Program cargo -Arguments @('check', '--locked', '--all-targets')
            Invoke-Checked -Program cargo -Arguments @('clippy', '--locked', '--all-targets', '--', '-D', 'warnings')
            if ($RunNative) {
                $nativeOutput = Join-Path $nativeMetadata.target_directory 'native-raster-apace.png'
                Invoke-Checked -Program cargo -Arguments @('run', '--locked', '--', $nativeOutput)
                Write-Host "Headless native artifacts: $nativeOutput and adjacent JSON report"
            }
        } finally { Pop-Location }
    }
    if ($RunMemory) {
        & "$PSScriptRoot/native-memory.ps1" -Cycles 2
        & "$PSScriptRoot/native-memory.ps1" -Cycles 2 -Raster
    }
    Write-Host 'Kartis checks passed.'
} finally {
    Pop-Location
}

