param(
    [string]$ConfigPath = "deploy/config/scheduler.properties",
    [string]$Host = "0.0.0.0",
    [int]$Port = 18080
)

mvn -DskipTests org.codehaus.mojo:exec-maven-plugin:3.5.0:java `
  -Dexec.mainClass=fuookami.ospf.framework.remote_solver.bootstrap.RemoteSolverApiMain `
  -Dexec.args="--config $ConfigPath --host $Host --port $Port"
