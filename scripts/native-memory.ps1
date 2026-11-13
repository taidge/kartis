param(
    [ValidateRange(1,3)][int]$Cycles = 2,
    [switch]$Raster,
    [switch]$SkipBuild,
    [string]$OutputPath
)
$ErrorActionPreference = 'Stop'
$probeRoot = Split-Path -Parent $PSScriptRoot
Push-Location $probeRoot
$probeProcess = $null
try {
    if (-not $IsWindows) { throw 'This sampler requires Windows OS process counters; it does not emulate RSS on other platforms.' }
    $metadataText = & cargo metadata --no-deps --format-version 1
    if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed' }
    $probeMetadata = $metadataText | ConvertFrom-Json
    if (-not $SkipBuild) {
        $buildArguments = @('build','--locked','--release','--example','memory_probe')
        if ($Raster) { $buildArguments += @('--features','raster') }
        & cargo @buildArguments
        if ($LASTEXITCODE -ne 0) { throw 'memory_probe build failed' }
    }
    $probeExecutable = Join-Path $probeMetadata.target_directory 'release/examples/memory_probe.exe'
    if (-not (Test-Path -LiteralPath $probeExecutable -PathType Leaf)) { throw "Probe executable absent: $probeExecutable" }
    $probeBinaryHash = (Get-FileHash -LiteralPath $probeExecutable -Algorithm SHA256).Hash
    if (-not $OutputPath) { $OutputPath = Join-Path $probeMetadata.target_directory $(if($Raster){'native-memory-raster.json'}else{'native-memory-default.json'}) }
    if (-not [System.IO.Path]::IsPathFullyQualified($OutputPath)) { throw 'OutputPath must be absolute' }
    $probeStart = [System.Diagnostics.ProcessStartInfo]::new()
    $probeStart.FileName = $probeExecutable
    $probeStart.WorkingDirectory = $probeRoot
    $probeStart.UseShellExecute = $false
    $probeStart.CreateNoWindow = $true
    $probeStart.RedirectStandardInput = $true
    $probeStart.RedirectStandardOutput = $true
    $probeStart.RedirectStandardError = $true
    $probeStart.ArgumentList.Add('--cycles')
    $probeStart.ArgumentList.Add($Cycles.ToString())
    $probeProcess = [System.Diagnostics.Process]::new()
    $probeProcess.StartInfo = $probeStart
    if (-not $probeProcess.Start()) { throw 'Could not start native memory probe' }
    $samples = [System.Collections.Generic.List[object]]::new()
    $seenBaseline = $false
    $seenComplete = $false
    while ($null -ne ($probeLine = $probeProcess.StandardOutput.ReadLine())) {
        $sample = $probeLine | ConvertFrom-Json
        if ($sample.pid -ne $probeProcess.Id) { throw 'Checkpoint PID does not match the owned process' }
        if ($sample.raster -ne [bool]$Raster) { throw 'Executable feature profile differs; rebuild without SkipBuild' }
        if ($sample.stage -eq 'baseline') { $seenBaseline = $true }
        if ($sample.stage -eq 'complete') { $seenComplete = $true }
        $probeProcess.Refresh()
        $sample | Add-Member -NotePropertyName sampledAt -NotePropertyValue ([DateTime]::UtcNow.ToString('o'))
        $sample | Add-Member -NotePropertyName workingSetBytes -NotePropertyValue $probeProcess.WorkingSet64
        $sample | Add-Member -NotePropertyName peakWorkingSetBytes -NotePropertyValue $probeProcess.PeakWorkingSet64
        $sample | Add-Member -NotePropertyName privateCommitBytes -NotePropertyValue $probeProcess.PrivateMemorySize64
        if ($sample.workingSetBytes -le 0 -or $sample.peakWorkingSetBytes -lt $sample.workingSetBytes) { throw 'Invalid OS working-set sample' }
        $samples.Add($sample)
        $probeProcess.StandardInput.WriteLine('sampled')
        $probeProcess.StandardInput.Flush()
    }
    $probeProcess.WaitForExit()
    $probeErrors = $probeProcess.StandardError.ReadToEnd()
    if ($probeProcess.ExitCode -ne 0) { throw "Native probe failed ($($probeProcess.ExitCode)): $probeErrors" }
    if ((Get-FileHash -LiteralPath $probeExecutable -Algorithm SHA256).Hash -ne $probeBinaryHash) { throw 'Probe binary changed during its run' }
    $expectedSamples = 2 + $Cycles * 4 * $(if($Raster){6}else{4})
    if (-not $seenBaseline -or -not $seenComplete -or $samples.Count -ne $expectedSamples) { throw 'Missing native memory checkpoints' }
    $report = [ordered]@{
        passed = $true
        sampledAt = [DateTime]::UtcNow.ToString('o')
        platform = [System.Runtime.InteropServices.RuntimeInformation]::OSDescription
        toolchain = (& rustc --version | Out-String).Trim()
        targetDirectory = $probeMetadata.target_directory
        executable = $probeExecutable
        executableSha256 = $probeBinaryHash
        probeSourceSha256 = (Get-FileHash -LiteralPath (Join-Path $probeRoot 'examples/memory_probe.rs') -Algorithm SHA256).Hash
        samplerSha256 = (Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash
        release = $true
        raster = [bool]$Raster
        cycles = $Cycles
        semantics = [ordered]@{
            workingSetBytes = 'OS resident working set of this one owned process; includes shared pages, executable and allocator retention, not live Rust allocations'
            peakWorkingSetBytes = 'OS high-water working set for this process lifetime; not a per-operation or sampled-only peak'
            privateCommitBytes = 'Windows private committed virtual memory; not resident physical bytes'
            heldOutputs = 'Checkpoints keep all earlier Scene/SVG/mesh plus the stated RGBA/PNG outputs alive until the parent acknowledges'
            limitations = 'Separate memory run with synchronous checkpoints; no throughput, allocation-count, leak-free, compositor or frame-rate claim; drop need not shrink allocator working set'
        }
        samples = $samples
    }
    $report | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $OutputPath -Encoding utf8
    Write-Output "Native memory samples: $($samples.Count); report: $OutputPath"
} finally {
    if ($probeProcess) {
        if (-not $probeProcess.HasExited) { $probeProcess.StandardInput.Close(); $probeProcess.WaitForExit(10000) | Out-Null }
        $probeProcess.Dispose()
    }
    Pop-Location
}
