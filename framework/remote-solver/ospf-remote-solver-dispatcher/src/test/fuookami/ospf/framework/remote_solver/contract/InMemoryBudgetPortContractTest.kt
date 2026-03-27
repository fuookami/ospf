package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryBudgetPort
import fuookami.ospf.framework.remote_solver.port.BudgetPort

class InMemoryBudgetPortContractTest : BudgetPortContractTest() {
    override fun createFixture(): TestFixture<BudgetPort> =
        TestFixture(
            subject = InMemoryBudgetPort()
        )
}
