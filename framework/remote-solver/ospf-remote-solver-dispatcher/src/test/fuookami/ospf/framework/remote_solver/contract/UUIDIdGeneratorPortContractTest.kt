package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.adapter.infrastructure.UUIDIdGeneratorPort
import fuookami.ospf.framework.remote_solver.protocol.port.IdGeneratorPort

class UUIDIdGeneratorPortContractTest : IdGeneratorPortContractTest() {
    override fun createFixture(): TestFixture<IdGeneratorPort> =
        TestFixture(
            subject = UUIDIdGeneratorPort()
        )
}
