param(
    [string]$ConfigPath = "deploy/config/scheduler.properties",
    [int]$Limit = 100
)

if (-not (Test-Path $ConfigPath)) {
    throw "Config file not found: $ConfigPath"
}

$properties = @{}
Get-Content $ConfigPath | ForEach-Object {
    $line = $_.Trim()
    if ([string]::IsNullOrWhiteSpace($line)) {
        return
    }
    if ($line.StartsWith("#")) {
        return
    }
    $index = $line.IndexOf("=")
    if ($index -le 0) {
        return
    }
    $key = $line.Substring(0, $index).Trim()
    $value = $line.Substring($index + 1).Trim()
    $properties[$key] = $value
}

$adapter = $properties["scheduler.audit.adapter"]
if ([string]::IsNullOrWhiteSpace($adapter)) {
    $adapter = "inmemory"
}

if ($adapter -ne "localfs") {
    Write-Host "scheduler.audit.adapter=$adapter. No local file to dump."
    exit 0
}

$auditRoot = $properties["scheduler.audit.localfs.path"]
if ([string]::IsNullOrWhiteSpace($auditRoot)) {
    $auditRoot = "target/remote-solver-audit"
}

$auditFile = Join-Path $auditRoot "audits.log"
if (-not (Test-Path $auditFile)) {
    Write-Host "Audit log not found: $auditFile"
    exit 0
}

Write-Host "config=$ConfigPath"
Write-Host "adapter=$adapter"
Write-Host "auditFile=$auditFile"
Write-Host "limit=$Limit"
Write-Host "----------------------------------------"

if ($Limit -gt 0) {
    Get-Content $auditFile -Tail $Limit
} else {
    Get-Content $auditFile
}
