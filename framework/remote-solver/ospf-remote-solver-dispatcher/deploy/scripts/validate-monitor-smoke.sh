#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
PORT=18089
BASE_URL="http://127.0.0.1:${PORT}"
PID_FILE="${PROJECT_ROOT}/target/monitor-smoke-validate.pid"

mkdir -p "${PROJECT_ROOT}/target"

bash -n "${SCRIPT_DIR}/monitor-smoke.sh"

python - <<'PY' &
from http.server import BaseHTTPRequestHandler, HTTPServer
import json

class Handler(BaseHTTPRequestHandler):
    @staticmethod
    def parse_roles(raw):
        if raw is None:
            return set()
        return {item.strip().lower() for item in raw.split(",") if item.strip()}

    def monitor_access(self):
        user_id = (self.headers.get("X-User-Id") or "").strip()
        if not user_id:
            return "unauthenticated"
        roles = self.parse_roles(self.headers.get("X-User-Roles"))
        if not roles.intersection({"admin", "monitor_read"}):
            return "forbidden"
        return "ok"

    def write_json(self, status, payload):
        body = json.dumps(payload).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        if self.path.startswith("/api/v1/monitor/overview"):
            access = self.monitor_access()
            if access == "unauthenticated":
                self.write_json(401, {"code": "UNAUTHENTICATED", "message": "authentication is required", "data": None})
                return
            if access == "forbidden":
                self.write_json(403, {"code": "MONITOR_ACCESS_DENIED", "message": "monitor access requires role", "data": None})
                return
            self.write_json(200, {
                "code": "OK",
                "message": "success",
                "data": {
                    "scheduler": {"schedulerConfigVersion": "v-test", "generatedAtEpochMs": 1712000000000, "nodeHeartbeatTimeoutMs": 30000},
                    "nodeTotals": {"totalNodes": 1},
                    "tasks": {"totalObservedTasks": 0, "queueDepth": 0, "runningTasks": 0, "failedTasks": 0, "completedTasks": 0, "statusCounts": {}, "recentTasks": []},
                    "nodes": []
                }
            })
            return
        if self.path == "/monitor":
            access = self.monitor_access()
            if access == "unauthenticated":
                self.send_response(302)
                self.send_header("Location", "/login")
                self.end_headers()
                return
            if access == "forbidden":
                self.write_json(403, {"code": "MONITOR_ACCESS_DENIED", "message": "monitor access requires role", "data": None})
                return
            body = b"<html><head><title>Remote Solver Monitor</title></head><body>ok</body></html>"
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            return
        self.send_response(404)
        self.end_headers()

    def log_message(self, fmt, *args):
        pass

HTTPServer(("127.0.0.1", 18089), Handler).serve_forever()
PY

SERVER_PID=$!
echo "${SERVER_PID}" > "${PID_FILE}"
cleanup() {
  if [[ -n "${SERVER_PID:-}" ]] && kill -0 "${SERVER_PID}" >/dev/null 2>&1; then
    kill "${SERVER_PID}" >/dev/null 2>&1 || true
    wait "${SERVER_PID}" 2>/dev/null || true
  fi
  rm -f "${PID_FILE}"
}
trap cleanup EXIT

sleep 1
"${SCRIPT_DIR}/monitor-smoke.sh" "${BASE_URL}" 120
OVERVIEW_UNAUTH_CODE="$(curl -s -o /dev/null -w "%{http_code}" "${BASE_URL}/api/v1/monitor/overview?limit=1")"
if [[ "${OVERVIEW_UNAUTH_CODE}" != "401" ]]; then
  echo "validation failed: expected unauthenticated overview status=401, got=${OVERVIEW_UNAUTH_CODE}"
  exit 1
fi
DASHBOARD_UNAUTH_CODE="$(curl -s -o /dev/null -w "%{http_code}" "${BASE_URL}/monitor")"
if [[ "${DASHBOARD_UNAUTH_CODE}" != "302" ]]; then
  echo "validation failed: expected unauthenticated dashboard status=302, got=${DASHBOARD_UNAUTH_CODE}"
  exit 1
fi
echo "validation passed: monitor-smoke.sh"
