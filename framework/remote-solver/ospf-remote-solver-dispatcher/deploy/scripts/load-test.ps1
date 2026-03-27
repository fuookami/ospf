# Remote Solver Capacity Test Script (Windows)
#
# This script runs capacity tests and generates reports.
#
# Usage:
#   .\load-test.ps1 [options]
#
# Options:
#   -Config PATH           Configuration file path (default: deploy\config\scheduler.properties)
#   -TotalTasks N          Total number of tasks (default: 100)
#   -Nodes N               Number of nodes (default: 6)
#   -SimpleRatio RATIO     Ratio of simple tasks (default: 0.7)
#   -Output FORMAT         Output format: text, json, both (default: text)
#   -OutputPath PATH       Output file path for JSON report
#   -SloSuccessRate R      SLO success rate target (default: 0.999)
#   -SloThroughput R       SLO throughput target (tasks/sec)
#   -Help                  Show this help message

param(
    [string]$Config = "",
    [int]$TotalTasks = 100,
    [int]$Nodes = 6,
    [double]$SimpleRatio = 0.7,
    [ValidateSet("text", "json", "both")]
    [string]$Output = "text",
    [string]$OutputPath = "",
    [double]$SloSuccessRate = 0.999,
    [double]$SloThroughput = 0,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent (Split-Path -Parent $ScriptDir)
$DefaultConfig = Join-Path $ProjectRoot "deploy\config\scheduler.properties"

if ($Help) {
    Get-Help $MyInvocation.MyCommand.Path -Detailed
    exit 0
}

# Use default config if not specified
if ([string]::IsNullOrEmpty($Config)) {
    $Config = $DefaultConfig
}

# Validate configuration file
if (-not (Test-Path $Config)) {
    Write-Host "ERROR: Configuration file not found: $Config" -ForegroundColor Red
    exit 1
}

Write-Host "=== Remote Solver Capacity Test ===" -ForegroundColor Cyan
Write-Host "Configuration: $Config"
Write-Host "Total tasks: $TotalTasks"
Write-Host "Nodes: $Nodes"
Write-Host "Simple ratio: $SimpleRatio"
Write-Host "Output format: $Output"
Write-Host ""

# Build command arguments
$Args = @(
    "--config", $Config,
    "--total-tasks", $TotalTasks,
    "--nodes", $Nodes,
    "--simple-ratio", $SimpleRatio,
    "--output", $Output,
    "--slo-success-rate", $SloSuccessRate
)

if (-not [string]::IsNullOrEmpty($OutputPath)) {
    $Args += @("--output-path", $OutputPath)
}

if ($SloThroughput -gt 0) {
    $Args += @("--slo-throughput", $SloThroughput)
}

# Run the capacity test
Set-Location $ProjectRoot

# Check if Maven is available
$MavenCmd = Get-Command mvn -ErrorAction SilentlyContinue
if ($null -ne $MavenCmd) {
    Write-Host "Running capacity test via Maven..."
    $MavenArgs = @(
        "-q", "exec:java",
        "-Dexec.mainClass=fuookami.ospf.framework.remote_solver.bootstrap.RemoteSolverLoadMain",
        "-Dexec.args=$($Args -join ' ')"
    )
    & mvn @MavenArgs
} else {
    Write-Host "ERROR: Maven not found. Please install Maven or run the test directly." -ForegroundColor Red
    exit 1
}
