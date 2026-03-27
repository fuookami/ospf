#!/usr/bin/env bash
set -euo pipefail

CONFIG_PATH="${1:-deploy/config/scheduler.properties}"
HOST="${2:-0.0.0.0}"
PORT="${3:-18080}"

mvn -DskipTests org.codehaus.mojo:exec-maven-plugin:3.5.0:java \
  -Dexec.mainClass=fuookami.ospf.framework.remote_solver.bootstrap.RemoteSolverApiMain \
  -Dexec.args="--config ${CONFIG_PATH} --host ${HOST} --port ${PORT}"
