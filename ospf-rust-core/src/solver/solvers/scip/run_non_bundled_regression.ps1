param(
    [string]$WorkspaceRoot = ""
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($WorkspaceRoot)) {
    $WorkspaceRoot = (Resolve-Path (Join-Path $PSScriptRoot "../../../../../")).Path
}

if (-not $env:SCIPOPTDIR) {
    Write-Error "SCIPOPTDIR is not set. Please set SCIPOPTDIR to your SCIP installation root."
}

if (-not (Test-Path $WorkspaceRoot)) {
    Write-Error "Workspace root not found: $WorkspaceRoot"
}

Push-Location $WorkspaceRoot
try {
    & (Join-Path $PSScriptRoot "check_non_bundled_env.ps1")

    Write-Host "[scip] workspace root: $WorkspaceRoot"
    Write-Host "[scip] SCIPOPTDIR: $env:SCIPOPTDIR"
    if ($env:LIBCLANG_PATH) {
        Write-Host "[scip] LIBCLANG_PATH: $env:LIBCLANG_PATH"
    }

    $commands = @(
        "cargo test -p ospf-rust-core --test scip_native_observer_integration --features scip -- --nocapture",
        "cargo test -p ospf-rust-framework --test scip_native_callback_integration --features scip -- --nocapture",
        "cargo test -p ospf-rust-framework --test scip_native_callback_async_integration --features `"scip async`" -- --nocapture"
    )

    foreach ($command in $commands) {
        Write-Host "[scip] running: $command"
        Invoke-Expression $command
        if ($LASTEXITCODE -ne 0) {
            Write-Error "Command failed: $command"
        }
    }

    Write-Host "[scip] non-bundled regression passed."
}
finally {
    Pop-Location
}
