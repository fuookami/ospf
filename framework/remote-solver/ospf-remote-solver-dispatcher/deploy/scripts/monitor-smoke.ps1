param(
    [string]$BaseUrl = "http://127.0.0.1:18080",
    [int]$Limit = 100,
    [string]$AuthUser = "smoke-user",
    [string]$AuthRoles = "monitor_read"
)

$overviewUrl = "$BaseUrl/api/v1/monitor/overview?limit=$Limit"
$dashboardUrl = "$BaseUrl/monitor"
$headers = @{
    "X-User-Id" = $AuthUser
    "X-User-Roles" = $AuthRoles
}

Write-Host "[monitor-smoke] check overview: $overviewUrl"
$overviewResponse = Invoke-WebRequest -Uri $overviewUrl -Headers $headers -UseBasicParsing
if ($overviewResponse.StatusCode -ne 200) {
    throw "[monitor-smoke] failed: overview status=$($overviewResponse.StatusCode)"
}
$overviewBody = $overviewResponse.Content
if ($overviewBody -notmatch '"code"\s*:\s*"OK"') {
    throw "[monitor-smoke] failed: overview api did not return code=OK"
}
if ($overviewBody -notmatch '"scheduler"' -or $overviewBody -notmatch '"nodes"') {
    throw "[monitor-smoke] failed: overview payload missing scheduler/nodes"
}

Write-Host "[monitor-smoke] check dashboard: $dashboardUrl"
$dashboardResponse = Invoke-WebRequest -Uri $dashboardUrl -Headers $headers -UseBasicParsing
if ($dashboardResponse.StatusCode -ne 200) {
    throw "[monitor-smoke] failed: dashboard status=$($dashboardResponse.StatusCode)"
}
if ($dashboardResponse.Content -notmatch '<title>Remote Solver Monitor</title>') {
    throw "[monitor-smoke] failed: dashboard title mismatch"
}

Write-Host "[monitor-smoke] success"
