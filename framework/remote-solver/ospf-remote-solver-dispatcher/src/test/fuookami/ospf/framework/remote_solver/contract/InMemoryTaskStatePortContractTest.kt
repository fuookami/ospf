package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryTaskStatePort
import fuookami.ospf.framework.remote_solver.port.TaskStatePort

class InMemoryTaskStatePortContractTest : TaskStatePortContractTest() {
    override fun createFixture(): TestFixture<TaskStatePort> =
        TestFixture(
            subject = InMemoryTaskStatePort()
        )
}
