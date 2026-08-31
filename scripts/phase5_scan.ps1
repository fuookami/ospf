Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Get-RgMatches {
    param(
        [Parameter(Mandatory = $true)][string]$Pattern,
        [Parameter(Mandatory = $true)][string]$Path
    )
    $output = & rg -n $Pattern $Path
    $exitCode = $LASTEXITCODE
    if ($exitCode -gt 1) {
        throw "rg failed for pattern '$Pattern' in path '$Path'"
    }
if ($exitCode -eq 1) {
        return @()
    }
    return @($output)
}

$legacyHits = Get-RgMatches -Pattern "crate::model::flatten|symbol::functions" -Path "ospf-rust-core/src"
$legacyHits = $legacyHits | Where-Object {
    $_ -notmatch "ospf-rust-core[\\/]+src[\\/]+symbol[\\/]+flatten[\\/]+mod\.rs" -and
    $_ -notmatch "ospf-rust-core[\\/]+src[\\/]+symbol[\\/]+functions[\\/]+mod\.rs"
}
if (@($legacyHits).Count -gt 0) {
    Write-Error "legacy path scan failed:`n$($legacyHits -join [Environment]::NewLine)"
    exit 1
}

$boundaryHits = Get-RgMatches -Pattern "to_f64_with_policy|from_f64_with_policy" -Path "ospf-rust-core/src/solver"
$boundaryHits = $boundaryHits | Where-Object {
    $_ -notmatch "ospf-rust-core[\\/]+src[\\/]+solver[\\/]+value[\\/]+boundary\.rs" -and
    $_ -notmatch "ospf-rust-core[\\/]+src[\\/]+solver[\\/]+value[\\/]+solve_value\.rs"
}
if (@($boundaryHits).Count -gt 0) {
    Write-Error "solver boundary scan failed:`n$($boundaryHits -join [Environment]::NewLine)"
    exit 1
}

$f64LeakHits = Get-RgMatches -Pattern "pub\\s+(fn|type|struct|enum).*f64" -Path "ospf-rust-core/src/solver/value"
$f64LeakHits = $f64LeakHits | Where-Object {
    $_ -notmatch "ospf-rust-core[\\/]+src[\\/]+solver[\\/]+value[\\/]+boundary\.rs" -and
    $_ -notmatch "ospf-rust-core[\\/]+src[\\/]+solver[\\/]+value[\\/]+solve_value\.rs"
}
if (@($f64LeakHits).Count -gt 0) {
    Write-Error "public API f64 leak scan failed:`n$($f64LeakHits -join [Environment]::NewLine)"
    exit 1
}

Write-Host "phase5 scans passed"
