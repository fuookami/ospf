param(
    [int]$ApiPort = 18091,
    [int]$WebhookPort = 19091
)

$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = Resolve-Path (Join-Path $scriptDir "..\..")
$targetDir = Join-Path $projectRoot "target\monitor-alert-linkage-validate"
$apiBaseUrl = "http://127.0.0.1:$ApiPort"
$webhookBaseUrl = "http://127.0.0.1:$WebhookPort"
$webhookEventsUrl = "$webhookBaseUrl/events"
$configPath = Join-Path $targetDir "scheduler-monitor-alert.properties"
$webhookScriptPath = Join-Path $targetDir "monitor-alert-webhook-mock.py"
$apiLog = Join-Path $targetDir "api.log"
$apiErrLog = Join-Path $targetDir "api.err.log"
$webhookLog = Join-Path $targetDir "webhook.log"
$webhookErrLog = Join-Path $targetDir "webhook.err.log"

New-Item -ItemType Directory -Path $targetDir -Force | Out-Null

@"
event.adapter=inmemory
distributed-lock.adapter=inmemory
node-state.adapter=inmemory
budget.adapter=inmemory
task-state.adapter=inmemory
cost-ledger.adapter=inmemory
solver-execution.adapter=inmemory
storage.adapter=inmemory
scheduler.loop.interval-ms=100
api.monitor.auth.enabled=false
monitor.alert.enabled=true
monitor.alert.check-interval-ms=0
monitor.alert.overview-limit=300
monitor.alert.cooldown-ms=0
monitor.alert.escalate-after-consecutive=2
monitor.alert.stale-nodes-threshold=0
monitor.alert.offline-nodes-threshold=0
monitor.alert.failed-tasks-threshold=0
monitor.alert.failed-ratio-threshold=0.3
monitor.alert.queue-depth-threshold=1
monitor.alert.route.event.enabled=true
monitor.alert.route.webhook.enabled=true
monitor.alert.route.webhook.url=$webhookBaseUrl/alerts
monitor.alert.route.webhook.timeout-ms=1000
"@ | Set-Content -Encoding UTF8 $configPath

@"
from http.server import BaseHTTPRequestHandler, HTTPServer
import json

events = []

class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == '/health':
            self.send_response(200)
            self.send_header('Content-Type', 'text/plain; charset=utf-8')
            self.end_headers()
            self.wfile.write(b'ok')
            return
        if self.path == '/events':
            body = json.dumps(events).encode('utf-8')
            self.send_response(200)
            self.send_header('Content-Type', 'application/json; charset=utf-8')
            self.send_header('Content-Length', str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            return
        self.send_response(404)
        self.end_headers()

    def do_POST(self):
        if self.path == '/alerts':
            size = int(self.headers.get('Content-Length', '0'))
            payload = self.rfile.read(size) if size > 0 else b'{}'
            events.append(json.loads(payload.decode('utf-8')))
            self.send_response(200)
            self.end_headers()
            return
        self.send_response(404)
        self.end_headers()

    def log_message(self, fmt, *args):
        pass

HTTPServer(('127.0.0.1', $WebhookPort), Handler).serve_forever()
"@ | Set-Content -Encoding UTF8 $webhookScriptPath

$webhookProcess = $null
$apiProcess = $null

function Stop-PortListeners {
    param([int]$Port)
    $connections = Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue
    if (-not $connections) {
        return
    }
    $pids = $connections | Select-Object -ExpandProperty OwningProcess -Unique
    foreach ($processId in $pids) {
        try {
            Stop-Process -Id $processId -Force -ErrorAction Stop
        } catch {
        }
    }
}

try {
    Stop-PortListeners -Port $ApiPort
    Stop-PortListeners -Port $WebhookPort

    $webhookProcess = Start-Process -FilePath "python" -ArgumentList $webhookScriptPath -PassThru -RedirectStandardOutput $webhookLog -RedirectStandardError $webhookErrLog
    for ($i = 0; $i -lt 80; $i++) {
        try {
            $health = Invoke-WebRequest -Uri "$webhookBaseUrl/health" -UseBasicParsing
            if ($health.StatusCode -eq 200) {
                break
            }
        } catch {
        }
        Start-Sleep -Milliseconds 500
    }
    $health = Invoke-WebRequest -Uri "$webhookBaseUrl/health" -UseBasicParsing
    if ($health.StatusCode -ne 200) {
        throw "validation failed: webhook mock did not start"
    }

    $apiArgs = @(
        "-DskipTests",
        "org.codehaus.mojo:exec-maven-plugin:3.5.0:java",
        "-Dexec.mainClass=fuookami.ospf.framework.remote_solver.bootstrap.RemoteSolverApiMain",
        "-Dexec.args=""--config $configPath --host 127.0.0.1 --port $ApiPort"""
    )
    $apiProcess = Start-Process -FilePath "mvn" -ArgumentList $apiArgs -PassThru -RedirectStandardOutput $apiLog -RedirectStandardError $apiErrLog

    $ready = $false
    for ($i = 0; $i -lt 120; $i++) {
        try {
            $response = Invoke-WebRequest -Uri "$apiBaseUrl/api/v1/tasks/non-exist" -UseBasicParsing
            if ($response.StatusCode -eq 404) {
                $ready = $true
                break
            }
        } catch {
            if ($_.Exception.Response -and $_.Exception.Response.StatusCode.value__ -eq 404) {
                $ready = $true
                break
            }
        }
        Start-Sleep -Milliseconds 500
    }
    if (-not $ready) {
        throw "validation failed: api service did not start"
    }

    $smokeScript = Join-Path $scriptDir "monitor-alert-smoke.ps1"
    $smokeArgs = @(
        "-File", $smokeScript,
        "-BaseUrl", $apiBaseUrl,
        "-WebhookEventsUrl", $webhookEventsUrl,
        "-TaskCount", "2",
        "-PollTimeoutSeconds", "60"
    )
    $smokeProcess = Start-Process -FilePath "pwsh" -ArgumentList $smokeArgs -PassThru -Wait -NoNewWindow
    if ($smokeProcess.ExitCode -ne 0) {
        throw "validation failed: monitor-alert-smoke.ps1 exitCode=$($smokeProcess.ExitCode)"
    }
    Write-Host "validation passed: monitor-alert-linkage"
}
finally {
    if ($apiProcess -and -not $apiProcess.HasExited) { try { Stop-Process -Id $apiProcess.Id -Force -ErrorAction Stop } catch {} }
    if ($webhookProcess -and -not $webhookProcess.HasExited) { try { Stop-Process -Id $webhookProcess.Id -Force -ErrorAction Stop } catch {} }
    Stop-PortListeners -Port $ApiPort
    Stop-PortListeners -Port $WebhookPort
}
