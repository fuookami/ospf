$ErrorActionPreference = "Stop"

if (-not $env:SCIPOPTDIR) {
    Write-Error "SCIPOPTDIR is not set. Please set SCIPOPTDIR to your SCIP installation root."
}

if (-not (Test-Path $env:SCIPOPTDIR)) {
    Write-Error "SCIPOPTDIR does not exist: $env:SCIPOPTDIR"
}

if ($env:LIBCLANG_PATH -and -not (Test-Path $env:LIBCLANG_PATH)) {
    Write-Error "LIBCLANG_PATH does not exist: $env:LIBCLANG_PATH"
}

$scipLib = Get-ChildItem -Path $env:SCIPOPTDIR -Recurse -File -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -in @("libscip.so", "libscip.dylib", "libscip.lib", "scip.lib") } |
    Select-Object -First 1

if (-not $scipLib) {
    Write-Error "No SCIP library file found under SCIPOPTDIR: $env:SCIPOPTDIR"
}

$scipHeader = Get-ChildItem -Path $env:SCIPOPTDIR -Recurse -File -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -eq "scip.h" } |
    Select-Object -First 1

if (-not $scipHeader) {
    Write-Error "No SCIP header file found under SCIPOPTDIR: $env:SCIPOPTDIR"
}

Write-Host "[scip] environment check passed."
Write-Host "[scip] SCIPOPTDIR: $env:SCIPOPTDIR"
if ($env:LIBCLANG_PATH) {
    Write-Host "[scip] LIBCLANG_PATH: $env:LIBCLANG_PATH"
}
