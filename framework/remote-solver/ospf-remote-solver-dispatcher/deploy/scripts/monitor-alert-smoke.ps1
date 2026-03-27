param(
    [string]$BaseUrl = "http://127.0.0.1:18091",
    [string]$WebhookEventsUrl = "http://127.0.0.1:19091/events",
    [int]$TaskCount = 2,
    [int]$PollTimeoutSeconds = 60
)

$submitBody = '{"payloadRef":"models/monitor-alert-smoke","complexity":"SIMPLE","timeSensitivity":"NON_REALTIME"}'

for ($i = 0; $i -lt $TaskCount; $i++) {
    $response = Invoke-WebRequest -Uri "$BaseUrl/api/v1/tasks" -Method Post -Body $submitBody -ContentType "application/json" -UseBasicParsing
    if ($response.StatusCode -ne 200 -or $response.Content -notmatch '"code"\s*:\s*"OK"') {
        throw "[monitor-alert-smoke] failed: submit api did not return code=OK"
    }
}

$deadline = (Get-Date).AddSeconds($PollTimeoutSeconds)
while ((Get-Date) -lt $deadline) {
    $events = (Invoke-WebRequest -Uri $WebhookEventsUrl -UseBasicParsing).Content
    if ($events -match "queue_depth" -and $events -match "TRIGGERED" -and $events -match "ESCALATED") {
        Write-Host "[monitor-alert-smoke] success"
        return
    }
    Start-Sleep -Seconds 1
}

throw "[monitor-alert-smoke] failed: expected queue_depth TRIGGERED/ESCALATED not found"
