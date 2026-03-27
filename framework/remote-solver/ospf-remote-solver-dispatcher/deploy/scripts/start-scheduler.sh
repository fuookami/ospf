#!/usr/bin/env bash
set -euo pipefail

CONFIG_PATH="${1:-deploy/config/scheduler.properties}"

mvn -DskipTests org.codehaus.mojo:exec-maven-plugin:3.5.0:java \
  -Dexec.mainClass=fuookami.ospf.framework.remote_solver.bootstrap.RemoteSolverSchedulerMain \
  -Dexec.args="--config ${CONFIG_PATH}"
