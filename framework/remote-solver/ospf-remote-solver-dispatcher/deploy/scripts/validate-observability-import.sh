#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
TARGET_DIR="${PROJECT_ROOT}/target/observability/ci-validate-rules"

bash -n "${SCRIPT_DIR}/import-observability.sh"

"${SCRIPT_DIR}/import-observability.sh" "http://127.0.0.1:3000" "" "${TARGET_DIR}"

if [[ ! -f "${TARGET_DIR}/alerts-remote-solver.yml" ]]; then
  echo "validation failed: expected alerts file is missing"
  exit 1
fi

echo "validation passed: import-observability.sh"

