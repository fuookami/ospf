#!/usr/bin/env bash
set -euo pipefail

CONFIG_PATH="${1:-deploy/config/scheduler.properties}"
LIMIT="${2:-100}"

if [[ ! -f "${CONFIG_PATH}" ]]; then
  echo "Config file not found: ${CONFIG_PATH}"
  exit 1
fi

ADAPTER="$(grep -E '^scheduler\.audit\.adapter=' "${CONFIG_PATH}" | tail -n 1 | cut -d'=' -f2- | xargs || true)"
if [[ -z "${ADAPTER}" ]]; then
  ADAPTER="inmemory"
fi

if [[ "${ADAPTER}" != "localfs" ]]; then
  echo "scheduler.audit.adapter=${ADAPTER}. No local file to dump."
  exit 0
fi

AUDIT_ROOT="$(grep -E '^scheduler\.audit\.localfs\.path=' "${CONFIG_PATH}" | tail -n 1 | cut -d'=' -f2- | xargs || true)"
if [[ -z "${AUDIT_ROOT}" ]]; then
  AUDIT_ROOT="target/remote-solver-audit"
fi

AUDIT_FILE="${AUDIT_ROOT}/audits.log"
if [[ ! -f "${AUDIT_FILE}" ]]; then
  echo "Audit log not found: ${AUDIT_FILE}"
  exit 0
fi

echo "config=${CONFIG_PATH}"
echo "adapter=${ADAPTER}"
echo "auditFile=${AUDIT_FILE}"
echo "limit=${LIMIT}"
echo "----------------------------------------"
if [[ "${LIMIT}" =~ ^[0-9]+$ ]] && [[ "${LIMIT}" -gt 0 ]]; then
  tail -n "${LIMIT}" "${AUDIT_FILE}"
else
  cat "${AUDIT_FILE}"
fi
