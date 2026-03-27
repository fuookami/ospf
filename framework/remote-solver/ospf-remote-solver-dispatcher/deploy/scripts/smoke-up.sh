#!/usr/bin/env bash
set -euo pipefail

CONFIG_PATH="${1:-deploy/config/scheduler.properties}"
LOG_DIR="${2:-target/smoke}"
SCHEDULER_LOG="${LOG_DIR}/scheduler.log"

mkdir -p "${LOG_DIR}"

cleanup() {
  if [[ -n "${SCHEDULER_PID:-}" ]] && kill -0 "${SCHEDULER_PID}" >/dev/null 2>&1; then
    kill "${SCHEDULER_PID}" >/dev/null 2>&1 || true
    wait "${SCHEDULER_PID}" 2>/dev/null || true
  fi
}
trap cleanup EXIT

echo "[smoke] start scheduler, config=${CONFIG_PATH}"
./deploy/scripts/start-scheduler.sh "${CONFIG_PATH}" > "${SCHEDULER_LOG}" 2>&1 &
SCHEDULER_PID=$!
sleep 2

echo "[smoke] run smoke main"
mvn -DskipTests org.codehaus.mojo:exec-maven-plugin:3.5.0:java \
  -Dexec.mainClass=fuookami.ospf.framework.remote_solver.bootstrap.RemoteSolverSmokeMain \
  -Dexec.args="--config ${CONFIG_PATH}"

echo "[smoke] success"
echo "[smoke] scheduler log: ${SCHEDULER_LOG}"
