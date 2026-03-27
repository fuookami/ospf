package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.adapter.infrastructure.SystemClockPort
import fuookami.ospf.framework.remote_solver.adapter.infrastructure.UUIDIdGeneratorPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryObjectStoragePort
import fuookami.ospf.framework.remote_solver.adapter.mock.MockSolverExecutionPort
import fuookami.ospf.framework.remote_solver.protocol.port.SolverExecutionPort

class OspfSolverExecutionPortContractTest : SolverExecutionPortContractTest() {
    override fun createFixture(): TestFixture<SolverExecutionPort> {
        val clock = SystemClockPort()
        val idGenerator = UUIDIdGeneratorPort()
        val objectStoragePort = InMemoryObjectStoragePort(clock)
        return TestFixture(
            subject = MockSolverExecutionPort(
                clock = clock,
                idGenerator = idGenerator,
                objectStoragePort = objectStoragePort,
                simulatedTotalRuntimeMs = 12_000L
            )
        )
    }
}
