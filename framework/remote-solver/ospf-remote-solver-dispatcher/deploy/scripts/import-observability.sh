#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

GRAFANA_URL="${1:-http://127.0.0.1:3000}"
GRAFANA_API_KEY="${2:-}"
PROM_RULES_TARGET_DIR="${3:-${PROJECT_ROOT}/target/observability/prometheus-rules}"

DASHBOARD_FILE="${PROJECT_ROOT}/deploy/observability/grafana/remote-solver-overview.json"
ALERTS_FILE="${PROJECT_ROOT}/deploy/observability/prometheus/alerts-remote-solver.yml"

mkdir -p "${PROM_RULES_TARGET_DIR}"
cp "${ALERTS_FILE}" "${PROM_RULES_TARGET_DIR}/alerts-remote-solver.yml"
echo "prometheus rules copied to ${PROM_RULES_TARGET_DIR}/alerts-remote-solver.yml"

if [[ -z "${GRAFANA_API_KEY}" ]]; then
  echo "grafana api key not provided, skip dashboard import"
  exit 0
fi

PAYLOAD_FILE="$(mktemp)"
{
  echo -n '{"dashboard":'
  cat "${DASHBOARD_FILE}"
  echo ',"folderId":0,"overwrite":true}'
} > "${PAYLOAD_FILE}"

HTTP_STATUS="$(
  curl -sS -o /dev/stderr -w "%{http_code}" \
    -X POST "${GRAFANA_URL%/}/api/dashboards/db" \
    -H "Authorization: Bearer ${GRAFANA_API_KEY}" \
    -H "Content-Type: application/json" \
    --data-binary "@${PAYLOAD_FILE}"
)"

rm -f "${PAYLOAD_FILE}"

if [[ "${HTTP_STATUS}" != "200" ]]; then
  echo "grafana dashboard import failed, status=${HTTP_STATUS}"
  exit 1
fi

echo "grafana dashboard imported successfully"

