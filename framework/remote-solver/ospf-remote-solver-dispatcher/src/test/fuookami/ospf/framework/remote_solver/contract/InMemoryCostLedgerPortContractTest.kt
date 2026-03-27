package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryCostLedgerPort
import fuookami.ospf.framework.remote_solver.port.CostLedgerPort

class InMemoryCostLedgerPortContractTest : CostLedgerPortContractTest() {
    override fun createFixture(): TestFixture<CostLedgerPort> =
        TestFixture(
            subject = InMemoryCostLedgerPort()
        )
}
