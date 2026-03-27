#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${1:-http://127.0.0.1:18080}"
LIMIT="${2:-100}"
AUTH_USER="${3:-smoke-user}"
AUTH_ROLES="${4:-monitor_read}"

OVERVIEW_URL="${BASE_URL}/api/v1/monitor/overview?limit=${LIMIT}"
DASHBOARD_URL="${BASE_URL}/monitor"

AUTH_HEADERS=(
  -H "X-User-Id: ${AUTH_USER}"
  -H "X-User-Roles: ${AUTH_ROLES}"
)

echo "[monitor-smoke] check overview: ${OVERVIEW_URL}"
OVERVIEW="$(curl -fsS "${AUTH_HEADERS[@]}" "${OVERVIEW_URL}")"
if [[ "${OVERVIEW}" != *"\"code\":\"OK\""* ]]; then
  echo "[monitor-smoke] failed: overview api did not return code=OK"
  exit 1
fi
if [[ "${OVERVIEW}" != *"\"scheduler\""* ]] || [[ "${OVERVIEW}" != *"\"nodes\""* ]]; then
  echo "[monitor-smoke] failed: overview payload missing scheduler/nodes"
  exit 1
fi

echo "[monitor-smoke] check dashboard: ${DASHBOARD_URL}"
DASHBOARD="$(curl -fsS "${AUTH_HEADERS[@]}" "${DASHBOARD_URL}")"
if [[ "${DASHBOARD}" != *"<title>Remote Solver Monitor</title>"* ]]; then
  echo "[monitor-smoke] failed: dashboard title mismatch"
  exit 1
fi

echo "[monitor-smoke] success"
