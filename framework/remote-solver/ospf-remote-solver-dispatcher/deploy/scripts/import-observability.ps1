param(
    [string]$GrafanaUrl = "http://127.0.0.1:3000",
    [string]$GrafanaApiKey = "",
    [string]$PromRulesTargetDir = ""
)

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = Resolve-Path (Join-Path $scriptDir "..\..")
if ([string]::IsNullOrWhiteSpace($PromRulesTargetDir)) {
    $PromRulesTargetDir = Join-Path $projectRoot "target\observability\prometheus-rules"
}

$dashboardFile = Join-Path $projectRoot "deploy\observability\grafana\remote-solver-overview.json"
$alertsFile = Join-Path $projectRoot "deploy\observability\prometheus\alerts-remote-solver.yml"

New-Item -ItemType Directory -Force -Path $PromRulesTargetDir | Out-Null
Copy-Item -Path $alertsFile -Destination (Join-Path $PromRulesTargetDir "alerts-remote-solver.yml") -Force
Write-Host "prometheus rules copied to $PromRulesTargetDir\alerts-remote-solver.yml"

if ([string]::IsNullOrWhiteSpace($GrafanaApiKey)) {
    Write-Host "grafana api key not provided, skip dashboard import"
    exit 0
}

$dashboardJson = Get-Content -Raw -Path $dashboardFile | ConvertFrom-Json
$payload = @{
    dashboard = $dashboardJson
    folderId = 0
    overwrite = $true
} | ConvertTo-Json -Depth 64

$headers = @{
    Authorization = "Bearer $GrafanaApiKey"
    "Content-Type" = "application/json"
}

$endpoint = "$($GrafanaUrl.TrimEnd('/'))/api/dashboards/db"
try {
    Invoke-RestMethod -Method Post -Uri $endpoint -Headers $headers -Body $payload | Out-Null
} catch {
    Write-Error "grafana dashboard import failed: $($_.Exception.Message)"
    exit 1
}

Write-Host "grafana dashboard imported successfully"

