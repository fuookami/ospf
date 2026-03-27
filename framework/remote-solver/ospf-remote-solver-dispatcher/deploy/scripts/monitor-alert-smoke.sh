#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${1:-http://127.0.0.1:18091}"
WEBHOOK_EVENTS_URL="${2:-http://127.0.0.1:19091/events}"
TASK_COUNT="${3:-2}"
POLL_TIMEOUT_SECONDS="${4:-60}"

SUBMIT_BODY='{"payloadRef":"models/monitor-alert-smoke","complexity":"SIMPLE","timeSensitivity":"NON_REALTIME"}'

for ((i = 0; i < TASK_COUNT; i++)); do
  RESPONSE="$(curl -fsS -X POST "${BASE_URL}/api/v1/tasks" -H "Content-Type: application/json" -d "${SUBMIT_BODY}")"
  if [[ "${RESPONSE}" != *"\"code\":\"OK\""* ]]; then
    echo "[monitor-alert-smoke] failed: submit api did not return code=OK"
    exit 1
  fi
done

DEADLINE=$(( $(date +%s) + POLL_TIMEOUT_SECONDS ))
while true; do
  EVENTS="$(curl -fsS "${WEBHOOK_EVENTS_URL}")"
  if [[ "${EVENTS}" == *"queue_depth"* ]] && [[ "${EVENTS}" == *"TRIGGERED"* ]] && [[ "${EVENTS}" == *"ESCALATED"* ]]; then
    echo "[monitor-alert-smoke] success"
    exit 0
  fi
  if (( $(date +%s) >= DEADLINE )); then
    echo "[monitor-alert-smoke] failed: expected queue_depth TRIGGERED/ESCALATED not found"
    echo "[monitor-alert-smoke] webhook events=${EVENTS}"
    exit 1
  fi
  sleep 1
done
