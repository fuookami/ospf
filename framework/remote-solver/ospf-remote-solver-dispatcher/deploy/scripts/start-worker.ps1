param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$WorkerArgs
)

$joined = [string]::Join(" ", $WorkerArgs)
mvn -DskipTests org.codehaus.mojo:exec-maven-plugin:3.5.0:java `
  -Dexec.mainClass=fuookami.ospf.framework.remote_solver.bootstrap.RemoteSolverWorkerMain `
  "-Dexec.args=$joined"
