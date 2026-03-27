package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryDistributedLockPort
import fuookami.ospf.framework.remote_solver.adapter.infrastructure.SystemClockPort
import fuookami.ospf.framework.remote_solver.adapter.infrastructure.UUIDIdGeneratorPort
import fuookami.ospf.framework.remote_solver.port.DistributedLockPort

class InMemoryDistributedLockPortContractTest : DistributedLockPortContractTest() {
    override fun createFixture(): TestFixture<DistributedLockPort> =
        TestFixture(
            subject = InMemoryDistributedLockPort(
                clock = SystemClockPort(),
                idGenerator = UUIDIdGeneratorPort()
            )
        )
}
