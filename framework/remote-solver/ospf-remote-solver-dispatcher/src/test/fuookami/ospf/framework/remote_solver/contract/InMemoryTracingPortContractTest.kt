package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryTracingPort
import fuookami.ospf.framework.remote_solver.port.TracingPort

class InMemoryTracingPortContractTest : TracingPortContractTest() {
    override fun createFixture(): TestFixture<TracingPort> =
        TestFixture(
            subject = InMemoryTracingPort()
        )
}
