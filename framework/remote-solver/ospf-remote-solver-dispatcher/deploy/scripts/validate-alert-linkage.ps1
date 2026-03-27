# Remote Solver Alert Linkage Validation Script (Windows)
#
# This script validates that alert configuration files are correctly formatted
# and that alert rules properly reference defined metrics.
#
# Usage:
#   .\validate-alert-linkage.ps1 [-Strict]
#
# Parameters:
#   -Strict  Exit with error if any warnings are found

param(
    [switch]$Strict
)

$ErrorActionPreference = "Continue"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent (Split-Path -Parent $ScriptDir)
$ObservabilityDir = Join-Path $ProjectRoot "deploy\observability"

Write-Host "=== Remote Solver Alert Linkage Validation ===" -ForegroundColor Cyan
Write-Host "Project root: $ProjectRoot"
Write-Host ""

$Errors = 0
$Warnings = 0

# Check 1: Prometheus alert rules file exists
Write-Host "[1/5] Checking Prometheus alert rules..."
$PrometheusRules = Join-Path $ObservabilityDir "prometheus\alerts-remote-solver.yml"
if (-not (Test-Path $PrometheusRules)) {
    Write-Host "ERROR: Prometheus alert rules file not found: $PrometheusRules" -ForegroundColor Red
    $Errors++
} else {
    Write-Host "  Found: $PrometheusRules" -ForegroundColor Green

    # Check required alert names
    $RequiredAlerts = @(
        "RemoteSolverHighFailureRate",
        "RemoteSolverSliceTimeoutSpike",
        "RemoteSolverNodeTimeoutRecoverySpike",
        "RemoteSolverCostSpike"
    )
    $RulesContent = Get-Content $PrometheusRules -Raw
    foreach ($AlertName in $RequiredAlerts) {
        if ($RulesContent -match "alert:\s*$AlertName") {
            Write-Host "  Alert '$AlertName': Found" -ForegroundColor Green
        } else {
            Write-Host "WARNING: Alert '$AlertName' not found in rules file" -ForegroundColor Yellow
            $Warnings++
        }
    }
}

Write-Host ""

# Check 2: AlertManager configuration file exists
Write-Host "[2/5] Checking AlertManager configuration..."
$AlertmanagerConfig = Join-Path $ObservabilityDir "alertmanager\alertmanager.yml"
if (-not (Test-Path $AlertmanagerConfig)) {
    Write-Host "ERROR: AlertManager config file not found: $AlertmanagerConfig" -ForegroundColor Red
    $Errors++
} else {
    Write-Host "  Found: $AlertmanagerConfig" -ForegroundColor Green

    # Check required receivers
    $RequiredReceivers = @("default-receiver", "critical-receiver", "warning-receiver")
    $ConfigContent = Get-Content $AlertmanagerConfig -Raw
    foreach ($Receiver in $RequiredReceivers) {
        if ($ConfigContent -match "name:\s*['`"]?$Receiver['`"]?") {
            Write-Host "  Receiver '$Receiver': Found" -ForegroundColor Green
        } else {
            Write-Host "WARNING: Receiver '$Receiver' not found in AlertManager config" -ForegroundColor Yellow
            $Warnings++
        }
    }
}

Write-Host ""

# Check 3: Metrics spec file exists
Write-Host "[3/5] Checking metrics specification..."
$MetricsSpec = Join-Path $ProjectRoot "docs\observability\metrics-spec.md"
if (-not (Test-Path $MetricsSpec)) {
    Write-Host "WARNING: Metrics spec file not found: $MetricsSpec" -ForegroundColor Yellow
    $Warnings++
} else {
    Write-Host "  Found: $MetricsSpec" -ForegroundColor Green
}

Write-Host ""

# Check 4: Alert-to-metric mapping
Write-Host "[4/5] Checking alert-to-metric mapping..."
if ((Test-Path $PrometheusRules) -and (Test-Path $MetricsSpec)) {
    $RulesContent = Get-Content $PrometheusRules -Raw
    $MetricsSpecContent = Get-Content $MetricsSpec -Raw

    # Extract metric names used in alerts
    $MetricPattern = "remote_solver_[a-z_]+"
    $Matches = [regex]::Matches($RulesContent, $MetricPattern)
    $UniqueMetrics = $Matches | ForEach-Object { $_.Value } | Select-Object -Unique

    foreach ($Metric in $UniqueMetrics) {
        if ($MetricsSpecContent -match [regex]::Escape($Metric)) {
            Write-Host "  Metric '$Metric': Documented" -ForegroundColor Green
        } else {
            Write-Host "WARNING: Metric '$Metric' used in alert but not documented in metrics-spec.md" -ForegroundColor Yellow
            $Warnings++
        }
    }
} else {
    Write-Host "  Skipped: Missing files" -ForegroundColor Yellow
}

Write-Host ""

# Check 5: Grafana dashboard exists
Write-Host "[5/5] Checking Grafana dashboard..."
$GrafanaDashboard = Join-Path $ObservabilityDir "grafana\remote-solver-overview.json"
if (-not (Test-Path $GrafanaDashboard)) {
    Write-Host "WARNING: Grafana dashboard not found: $GrafanaDashboard" -ForegroundColor Yellow
    $Warnings++
} else {
    Write-Host "  Found: $GrafanaDashboard" -ForegroundColor Green

    $DashboardContent = Get-Content $GrafanaDashboard -Raw
    if ($DashboardContent -match "remote_solver_task") {
        Write-Host "  Task metrics: Referenced" -ForegroundColor Green
    } else {
        Write-Host "WARNING: Grafana dashboard does not reference task metrics" -ForegroundColor Yellow
        $Warnings++
    }
}

Write-Host ""
Write-Host "=== Validation Summary ===" -ForegroundColor Cyan
Write-Host "Errors: $Errors"
Write-Host "Warnings: $Warnings"
Write-Host ""

if ($Errors -gt 0) {
    Write-Host "FAIL: Alert linkage validation failed with $Errors error(s)" -ForegroundColor Red
    exit 1
}

if ($Strict -and $Warnings -gt 0) {
    Write-Host "FAIL: Strict mode enabled, validation failed with $Warnings warning(s)" -ForegroundColor Red
    exit 1
}

Write-Host "PASS: Alert linkage validation completed successfully" -ForegroundColor Green
exit 0