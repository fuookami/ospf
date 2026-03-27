package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryObjectStoragePort
import fuookami.ospf.framework.remote_solver.adapter.mock.MockSolverExecutionPort
import fuookami.ospf.framework.remote_solver.adapter.infrastructure.SystemClockPort
import fuookami.ospf.framework.remote_solver.adapter.infrastructure.UUIDIdGeneratorPort
import fuookami.ospf.framework.remote_solver.protocol.port.SolverExecutionPort

/**
 * Contract test for MockSolverExecutionPort.
 *
 * Note: This tests the mock implementation, not a real solver.
 * For real solver tests, use OspfSolverExecutionPortContractTest.
 */
class InMemorySolverExecutionPortContractTest : SolverExecutionPortContractTest() {
    override fun createFixture(): TestFixture<SolverExecutionPort> {
        val clock = SystemClockPort()
        val objectStoragePort = InMemoryObjectStoragePort(clock)
        return TestFixture(
            subject = MockSolverExecutionPort(
                clock = clock,
                idGenerator = UUIDIdGeneratorPort(),
                objectStoragePort = objectStoragePort
            )
        )
    }
}
