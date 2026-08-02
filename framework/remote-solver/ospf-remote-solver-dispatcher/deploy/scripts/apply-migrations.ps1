param(
    [Parameter(Mandatory = $true)]
    [string]$DbUrl
)

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$rootDir = Resolve-Path (Join-Path $scriptDir "..\..")
$v1 = Join-Path $rootDir "deploy\sql\V1__remote_solver_core.sql"
$v2 = Join-Path $rootDir "deploy\sql\V2__remote_solver_infra.sql"
$v3 = Join-Path $rootDir "deploy\sql\V3__remote_solver_scheduler_audit.sql"
$v4 = Join-Path $rootDir "deploy\sql\V4__remote_solver_multi_tenant.sql"
$v5 = Join-Path $rootDir "deploy\sql\V5__remote_solver_cp2.sql"
$v6 = Join-Path $rootDir "deploy\sql\V6__remote_solver_cp2_payload_config.sql"
$v7 = Join-Path $rootDir "deploy\sql\V7__remote_solver_object_ref_etag.sql"

psql $DbUrl -f $v1
if ($LASTEXITCODE -ne 0) {
    throw "Failed to apply V1 migration."
}

psql $DbUrl -f $v2
if ($LASTEXITCODE -ne 0) {
    throw "Failed to apply V2 migration."
}

psql $DbUrl -f $v3
if ($LASTEXITCODE -ne 0) {
    throw "Failed to apply V3 migration."
}

psql $DbUrl -f $v4
if ($LASTEXITCODE -ne 0) {
    throw "Failed to apply V4 migration."
}

psql $DbUrl -f $v5
if ($LASTEXITCODE -ne 0) {
    throw "Failed to apply V5 migration."
}

psql $DbUrl -f $v6
if ($LASTEXITCODE -ne 0) {
    throw "Failed to apply V6 migration."
}

psql $DbUrl -f $v7
if ($LASTEXITCODE -ne 0) {
    throw "Failed to apply V7 migration."
}

Write-Host "Applied migrations: V1, V2, V3, V4, V5, V6, V7"
