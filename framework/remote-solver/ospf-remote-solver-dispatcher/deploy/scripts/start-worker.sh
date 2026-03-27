#!/usr/bin/env bash
set -euo pipefail

mvn -DskipTests org.codehaus.mojo:exec-maven-plugin:3.5.0:java \
  -Dexec.mainClass=fuookami.ospf.framework.remote_solver.bootstrap.RemoteSolverWorkerMain \
  -Dexec.args="$*"
