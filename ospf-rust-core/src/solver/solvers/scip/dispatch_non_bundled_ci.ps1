param(
    [ValidateSet("linux", "windows")]
    [string]$TargetOS = "windows",
    [string]$Ref = "master",
    [string]$Owner = "fuookami",
    [string]$Repo = "ospf-rust",
    [string]$Workflow = "scip-non-bundled-self-hosted.yml",
    [string]$Token = "",
    [switch]$Wait,
    [int]$TimeoutSeconds = 1800,
    [int]$PollIntervalSeconds = 10
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($Token)) {
    $Token = $env:GITHUB_TOKEN
}
if ([string]::IsNullOrWhiteSpace($Token)) {
    $Token = $env:GH_TOKEN
}
if ([string]::IsNullOrWhiteSpace($Token)) {
    throw "Missing token. Set GITHUB_TOKEN/GH_TOKEN or pass -Token."
}

$base = "https://api.github.com/repos/$Owner/$Repo/actions"
$headers = @{
    Authorization = "Bearer $Token"
    Accept        = "application/vnd.github+json"
    "X-GitHub-Api-Version" = "2022-11-28"
}

$dispatchBody = @{
    ref = $Ref
    inputs = @{
        target_os = $TargetOS
    }
} | ConvertTo-Json -Depth 4

$startUtc = [DateTime]::UtcNow
Invoke-RestMethod -Method Post -Uri "$base/workflows/$Workflow/dispatches" -Headers $headers -Body $dispatchBody -ContentType "application/json" | Out-Null
Write-Host "[scip-ci] dispatched: workflow=$Workflow target_os=$TargetOS ref=$Ref"

if (-not $Wait) {
    exit 0
}

$deadline = (Get-Date).AddSeconds($TimeoutSeconds)
while ((Get-Date) -lt $deadline) {
    $runs = Invoke-RestMethod -Method Get -Uri "$base/workflows/$Workflow/runs?event=workflow_dispatch&branch=$Ref&per_page=20" -Headers $headers
    $run = $runs.workflow_runs |
        Where-Object { [DateTime]$_.created_at -ge $startUtc.AddMinutes(-1) } |
        Sort-Object { [DateTime]$_.created_at } -Descending |
        Select-Object -First 1

    if ($null -ne $run) {
        Write-Host "[scip-ci] run id=$($run.id) status=$($run.status) conclusion=$($run.conclusion) url=$($run.html_url)"
        if ($run.status -eq "completed") {
            if ($run.conclusion -ne "success") {
                throw "Workflow completed with conclusion: $($run.conclusion). URL: $($run.html_url)"
            }
            exit 0
        }
    }

    Start-Sleep -Seconds $PollIntervalSeconds
}

throw "Timeout waiting workflow completion ($TimeoutSeconds seconds)."
