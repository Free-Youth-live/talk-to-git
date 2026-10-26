param(
    [int]$Iterations = 5,
    [int]$TimeoutSeconds = 30
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$outputDirectory = Join-Path $root 'artifacts'
New-Item -ItemType Directory -Force -Path $outputDirectory | Out-Null
$results = @()
$prototypes = @(
    @{ Name = 'egui'; Executable = Join-Path $root 'target/release/ttg-gui.exe' }
)

foreach ($prototype in $prototypes) {
    foreach ($fixture in @('standard', 'large-diff', 'history')) {
        $repo = Join-Path $root "fixtures/generated/$fixture"
        for ($iteration = 1; $iteration -le $Iterations; $iteration++) {
            $stdout = Join-Path $outputDirectory "$($prototype.Name)-$fixture-startup-$iteration.stdout.log"
            $stderr = Join-Path $outputDirectory "$($prototype.Name)-$fixture-startup-$iteration.stderr.log"
            $started = Get-Date
            $process = Start-Process -FilePath $prototype.Executable `
                -ArgumentList @('--repo', $repo, '--benchmark-startup') `
                -RedirectStandardOutput $stdout `
                -RedirectStandardError $stderr `
                -PassThru
            if (!$process.WaitForExit($TimeoutSeconds * 1000)) {
                $process.Kill()
                $process.WaitForExit()
                throw "$($prototype.Name) startup benchmark timed out for $fixture run $iteration"
            }
            $ended = Get-Date
            $process.Refresh()
            $exitCode = $process.ExitCode
            if ($null -eq $exitCode) {
                $exitCode = 0
            }
            if ($exitCode -ne 0) {
                $details = @($stdout, $stderr) | ForEach-Object {
                    if (Test-Path $_) { Get-Content -Raw -ErrorAction SilentlyContinue $_ }
                }
                throw "$($prototype.Name) exited with $exitCode for ${fixture} run ${iteration}: $details"
            }
            $readyLine = Get-Content $stdout -ErrorAction SilentlyContinue |
                Where-Object { $_ -match '^READY_MS=(\d+)$' } |
                Select-Object -Last 1
            if (!$readyLine) {
                throw "$($prototype.Name) did not report READY_MS for $fixture run $iteration"
            }
            $null = $readyLine -match '^READY_MS=(\d+)$'
            $results += [PSCustomObject]@{
                prototype = $prototype.Name
                fixture = $fixture
                iteration = $iteration
                measuredAt = $started.ToString('o')
                wallClockMs = [Math]::Round(($ended - $started).TotalMilliseconds)
                readyMs = [int]$Matches[1]
                exitCode = $exitCode
            }
        }
    }
}

$output = Join-Path $outputDirectory 'windows-startup.json'
$results | ConvertTo-Json -Depth 3 | Set-Content -Path $output -Encoding UTF8
$results | Format-Table prototype, fixture, iteration, wallClockMs, readyMs -AutoSize
