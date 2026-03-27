package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.adapter.infrastructure.SystemClockPort
import fuookami.ospf.framework.remote_solver.protocol.port.ClockPort

class SystemClockPortContractTest : ClockPortContractTest() {
    override fun createFixture(): TestFixture<ClockPort> =
        TestFixture(
            subject = SystemClockPort()
        )
}
