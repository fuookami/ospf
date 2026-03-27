package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryNodeStatePort
import fuookami.ospf.framework.remote_solver.port.NodeStatePort

class InMemoryNodeStatePortContractTest : NodeStatePortContractTest() {
    override fun createFixture(): TestFixture<NodeStatePort> =
        TestFixture(
            subject = InMemoryNodeStatePort()
        )
}
