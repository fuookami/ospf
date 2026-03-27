#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
API_PORT=18091
WEBHOOK_PORT=19091
API_BASE_URL="http://127.0.0.1:${API_PORT}"
WEBHOOK_BASE_URL="http://127.0.0.1:${WEBHOOK_PORT}"
WEBHOOK_EVENTS_URL="${WEBHOOK_BASE_URL}/events"
TARGET_DIR="${PROJECT_ROOT}/target/monitor-alert-linkage-validate"
CONFIG_PATH="${TARGET_DIR}/scheduler-monitor-alert.properties"
API_LOG="${TARGET_DIR}/api.log"
WEBHOOK_LOG="${TARGET_DIR}/webhook.log"

mkdir -p "${TARGET_DIR}"
: > "${API_LOG}"
: > "${WEBHOOK_LOG}"

bash -n "${SCRIPT_DIR}/monitor-alert-smoke.sh"

cat > "${CONFIG_PATH}" <<EOF
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
monitor.alert.route.webhook.url=${WEBHOOK_BASE_URL}/alerts
monitor.alert.route.webhook.timeout-ms=1000
EOF

WEBHOOK_PORT="${WEBHOOK_PORT}" python - <<'PY' > "${WEBHOOK_LOG}" 2>&1 &
from http.server import BaseHTTPRequestHandler, HTTPServer
import json
import os

events = []

class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/health":
            self.send_response(200)
            self.send_header("Content-Type", "text/plain; charset=utf-8")
            self.end_headers()
            self.wfile.write(b"ok")
            return
        if self.path == "/events":
            body = json.dumps(events).encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "application/json; charset=utf-8")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            return
        self.send_response(404)
        self.end_headers()

    def do_POST(self):
        if self.path == "/alerts":
            size = int(self.headers.get("Content-Length", "0"))
            payload = self.rfile.read(size) if size > 0 else b"{}"
            events.append(json.loads(payload.decode("utf-8")))
            self.send_response(200)
            self.end_headers()
            return
        self.send_response(404)
        self.end_headers()

    def log_message(self, fmt, *args):
        pass

port = int(os.environ["WEBHOOK_PORT"])
HTTPServer(("127.0.0.1", port), Handler).serve_forever()
PY
WEBHOOK_PID=$!

cleanup() {
  if [[ -n "${API_PID:-}" ]] && kill -0 "${API_PID}" >/dev/null 2>&1; then
    kill "${API_PID}" >/dev/null 2>&1 || true
    wait "${API_PID}" 2>/dev/null || true
  fi
  if [[ -n "${WEBHOOK_PID:-}" ]] && kill -0 "${WEBHOOK_PID}" >/dev/null 2>&1; then
    kill "${WEBHOOK_PID}" >/dev/null 2>&1 || true
    wait "${WEBHOOK_PID}" 2>/dev/null || true
  fi
}
trap cleanup EXIT

for _ in $(seq 1 40); do
  if curl -fsS "${WEBHOOK_BASE_URL}/health" >/dev/null 2>&1; then
    break
  fi
  sleep 0.5
done
if ! curl -fsS "${WEBHOOK_BASE_URL}/health" >/dev/null 2>&1; then
  echo "validation failed: webhook mock did not start"
  exit 1
fi

mvn -DskipTests org.codehaus.mojo:exec-maven-plugin:3.5.0:java \
  -Dexec.mainClass=fuookami.ospf.framework.remote_solver.bootstrap.RemoteSolverApiMain \
  -Dexec.args="--config ${CONFIG_PATH} --host 127.0.0.1 --port ${API_PORT}" \
  > "${API_LOG}" 2>&1 &
API_PID=$!

for _ in $(seq 1 80); do
  code="$(curl -s -o /dev/null -w "%{http_code}" "${API_BASE_URL}/api/v1/tasks/non-exist")"
  if [[ "${code}" == "404" ]]; then
    break
  fi
  sleep 0.5
done
code="$(curl -s -o /dev/null -w "%{http_code}" "${API_BASE_URL}/api/v1/tasks/non-exist")"
if [[ "${code}" != "404" ]]; then
  echo "validation failed: api service did not start, status=${code}"
  exit 1
fi

"${SCRIPT_DIR}/monitor-alert-smoke.sh" "${API_BASE_URL}" "${WEBHOOK_EVENTS_URL}" 2 60
echo "validation passed: monitor-alert-linkage"
