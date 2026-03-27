package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryObjectStoragePort
import fuookami.ospf.framework.remote_solver.adapter.infrastructure.SystemClockPort
import fuookami.ospf.framework.remote_solver.protocol.port.ObjectStoragePort

class InMemoryObjectStoragePortContractTest : ObjectStoragePortContractTest() {
    override fun createFixture(): TestFixture<ObjectStoragePort> =
        TestFixture(
            subject = InMemoryObjectStoragePort(SystemClockPort())
        )
}
