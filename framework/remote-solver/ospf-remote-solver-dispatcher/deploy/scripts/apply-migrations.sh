#!/usr/bin/env bash
set -euo pipefail

DB_URL="${1:-}"
if [[ -z "${DB_URL}" ]]; then
  echo "Usage: ./deploy/scripts/apply-migrations.sh <postgresql-url>"
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

psql "${DB_URL}" -f "${ROOT_DIR}/deploy/sql/V1__remote_solver_core.sql"
psql "${DB_URL}" -f "${ROOT_DIR}/deploy/sql/V2__remote_solver_infra.sql"
psql "${DB_URL}" -f "${ROOT_DIR}/deploy/sql/V3__remote_solver_scheduler_audit.sql"
psql "${DB_URL}" -f "${ROOT_DIR}/deploy/sql/V4__remote_solver_multi_tenant.sql"
psql "${DB_URL}" -f "${ROOT_DIR}/deploy/sql/V5__remote_solver_cp2.sql"
psql "${DB_URL}" -f "${ROOT_DIR}/deploy/sql/V6__remote_solver_cp2_payload_config.sql"
psql "${DB_URL}" -f "${ROOT_DIR}/deploy/sql/V7__remote_solver_object_ref_etag.sql"

echo "Applied migrations: V1, V2, V3, V4, V5, V6, V7"
