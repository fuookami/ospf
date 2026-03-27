package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.adapter.infrastructure.SystemClockPort
import fuookami.ospf.framework.remote_solver.adapter.infrastructure.UUIDIdGeneratorPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryObjectStoragePort
import fuookami.ospf.framework.remote_solver.adapter.ospf.OspfExternalProcessBridge
import fuookami.ospf.framework.remote_solver.adapter.ospf.OspfSolverExecutionPort
import fuookami.ospf.framework.remote_solver.protocol.port.SolverExecutionPort

class OspfExternalProcessBridgeContractTest : SolverExecutionPortContractTest() {
    override fun createFixture(): TestFixture<SolverExecutionPort> {
        val clock = SystemClockPort()
        val idGenerator = UUIDIdGeneratorPort()
        val objectStoragePort = InMemoryObjectStoragePort(clock)
        val bridge = OspfExternalProcessBridge(
            clock = clock,
            idGenerator = idGenerator,
            objectStoragePort = objectStoragePort,
            args = mapOf(
                "command" to "cmd /c \"echo completed=true && echo feasible=true && echo objective=1.0 && echo gap=0.0 && echo elapsedMs=100 && echo checkpointPath=cp.bin && echo resultPath=res.bin\""
            )
        )
        return TestFixture(
            subject = OspfSolverExecutionPort(bridge)
        )
    }
}
