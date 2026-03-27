param(
    [string]$ConfigPath = "deploy/config/scheduler.properties",
    [string]$LogDir = "target/smoke"
)

$null = New-Item -ItemType Directory -Path $LogDir -Force
$schedulerLog = Join-Path $LogDir "scheduler.log"

Write-Host "[smoke] start scheduler, config=$ConfigPath"
$schedulerProc = Start-Process -FilePath "powershell" `
    -ArgumentList @(
        "-NoProfile",
        "-ExecutionPolicy", "Bypass",
        "-File", "deploy/scripts/start-scheduler.ps1",
        "-ConfigPath", $ConfigPath
    ) `
    -RedirectStandardOutput $schedulerLog `
    -RedirectStandardError $schedulerLog `
    -PassThru

Start-Sleep -Seconds 2

try {
    Write-Host "[smoke] run smoke main"
    mvn -DskipTests org.codehaus.mojo:exec-maven-plugin:3.5.0:java `
      -Dexec.mainClass=fuookami.ospf.framework.remote_solver.bootstrap.RemoteSolverSmokeMain `
      "-Dexec.args=--config $ConfigPath"
    Write-Host "[smoke] success"
    Write-Host "[smoke] scheduler log: $schedulerLog"
} finally {
    if ($schedulerProc -and -not $schedulerProc.HasExited) {
        Stop-Process -Id $schedulerProc.Id -Force
    }
}
