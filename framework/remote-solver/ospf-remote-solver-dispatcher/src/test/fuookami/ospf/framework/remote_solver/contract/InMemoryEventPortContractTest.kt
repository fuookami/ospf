package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryEventPort
import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryEventRetryPolicy
import fuookami.ospf.framework.remote_solver.adapter.infrastructure.SystemClockPort
import fuookami.ospf.framework.remote_solver.adapter.infrastructure.UUIDIdGeneratorPort
import fuookami.ospf.framework.remote_solver.port.EventPort

class InMemoryEventPortContractTest : EventPortContractTest() {
    override fun createFixture(): TestFixture<EventPort> =
        TestFixture(
            subject = InMemoryEventPort(
                clock = SystemClockPort(),
                idGenerator = UUIDIdGeneratorPort(),
                retryPolicy = InMemoryEventRetryPolicy(baseDelayMs = 0L)
            )
        )
}
