#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
TMP_DIR="${PROJECT_ROOT}/target/scheduler-audit-dump-validate"

mkdir -p "${TMP_DIR}"

bash -n "${SCRIPT_DIR}/scheduler-audit-dump.sh"

LOCALFS_CONFIG="${TMP_DIR}/scheduler-localfs.properties"
LOCALFS_AUDIT_ROOT="${TMP_DIR}/localfs-audit"
mkdir -p "${LOCALFS_AUDIT_ROOT}"
cat > "${LOCALFS_CONFIG}" <<EOF
scheduler.audit.adapter=localfs
scheduler.audit.localfs.path=${LOCALFS_AUDIT_ROOT}
EOF
cat > "${LOCALFS_AUDIT_ROOT}/audits.log" <<EOF
version=v1 operator=ops effectiveAt=1712000000000
version=v2 operator=ops effectiveAt=1712000001000
EOF

LOCALFS_OUTPUT="$("${SCRIPT_DIR}/scheduler-audit-dump.sh" "${LOCALFS_CONFIG}" 1)"
if [[ "${LOCALFS_OUTPUT}" != *"adapter=localfs"* ]]; then
  echo "validation failed: expected localfs adapter output"
  exit 1
fi
if [[ "${LOCALFS_OUTPUT}" != *"version=v2"* ]]; then
  echo "validation failed: expected latest audit line for limit=1"
  exit 1
fi
if [[ "${LOCALFS_OUTPUT}" == *"version=v1"* ]]; then
  echo "validation failed: expected only tail line when limit=1"
  exit 1
fi

INMEMORY_CONFIG="${TMP_DIR}/scheduler-inmemory.properties"
cat > "${INMEMORY_CONFIG}" <<EOF
scheduler.audit.adapter=inmemory
EOF

INMEMORY_OUTPUT="$("${SCRIPT_DIR}/scheduler-audit-dump.sh" "${INMEMORY_CONFIG}" 10)"
if [[ "${INMEMORY_OUTPUT}" != *"scheduler.audit.adapter=inmemory. No local file to dump."* ]]; then
  echo "validation failed: expected inmemory no-op output"
  exit 1
fi

echo "validation passed: scheduler-audit-dump.sh"
