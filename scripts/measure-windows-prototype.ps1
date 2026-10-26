param(
    [Parameter(Mandatory = $true)][string]$Executable,
    [Parameter(Mandatory = $true)][string]$Name
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$outputDirectory = Join-Path $root 'artifacts'
New-Item -ItemType Directory -Force -Path $outputDirectory | Out-Null
$executablePath = (Resolve-Path $Executable).Path
$results = @()

function Get-ProcessTreeIds {
    param([int]$RootId, [array]$Processes)
    $ids = @($RootId)
    foreach ($child in @($Processes | Where-Object { $_.ParentProcessId -eq $RootId })) {
        $ids += Get-ProcessTreeIds -RootId $child.ProcessId -Processes $Processes
    }
    return $ids
}

foreach ($fixture in @('standard', 'large-diff', 'history')) {
    $repo = Join-Path $root "fixtures/generated/$fixture"
    $started = Get-Date
    $stdout = Join-Path $outputDirectory "$Name-$fixture.stdout.log"
    $stderr = Join-Path $outputDirectory "$Name-$fixture.stderr.log"
    $process = Start-Process -FilePath $executablePath -ArgumentList @('--repo', $repo) -RedirectStandardOutput $stdout -RedirectStandardError $stderr -PassThru
    Start-Sleep -Seconds 6
    $process.Refresh()
    if ($process.HasExited) {
        $details = @($stdout, $stderr) | ForEach-Object {
            if (Test-Path $_) { Get-Content -Raw -ErrorAction SilentlyContinue $_ }
        }
        throw "$Name exited early for $fixture with code $($process.ExitCode): $details"
    }
    $allProcesses = @(Get-CimInstance Win32_Process)
    $treeIds = Get-ProcessTreeIds -RootId $process.Id -Processes $allProcesses
    $tree = @(Get-Process -Id $treeIds -ErrorAction SilentlyContinue)
    $cpuSeconds = 0.0
    foreach ($item in $tree) {
        $cpuSeconds += $item.TotalProcessorTime.TotalSeconds
    }
    $results += [PSCustomObject]@{
        prototype = $Name
        fixture = $fixture
        measuredAt = $started.ToString('o')
        processCount = $tree.Count
        rootWorkingSet64 = $process.WorkingSet64
        treeWorkingSet64 = ($tree | Measure-Object WorkingSet64 -Sum).Sum
        treePrivateMemorySize64 = ($tree | Measure-Object PrivateMemorySize64 -Sum).Sum
        treeCpuSeconds = [Math]::Round($cpuSeconds, 3)
    }
    $null = $process.CloseMainWindow()
    Start-Sleep -Seconds 2
    if (!$process.HasExited) {
        $process.Kill()
        $process.WaitForExit()
    }
}

$output = Join-Path $outputDirectory "$Name-windows-memory.json"
$results | ConvertTo-Json -Depth 3 | Set-Content -Path $output -Encoding UTF8
$results | Format-Table -AutoSize
