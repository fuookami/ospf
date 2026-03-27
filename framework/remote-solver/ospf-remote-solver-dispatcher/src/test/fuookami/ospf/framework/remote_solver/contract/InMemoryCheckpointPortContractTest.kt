package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryCheckpointPort
import fuookami.ospf.framework.remote_solver.protocol.port.CheckpointPort

class InMemoryCheckpointPortContractTest : CheckpointPortContractTest() {
    override fun createFixture(): TestFixture<CheckpointPort> =
        TestFixture(
            subject = InMemoryCheckpointPort()
        )
}
