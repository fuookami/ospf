package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.adapter.ktorm.KtormBudgetPort
import fuookami.ospf.framework.remote_solver.port.BudgetPort

class KtormBudgetPortContractTest : BudgetPortContractTest() {
    override fun createFixture(): TestFixture<BudgetPort> {
        val dbName = "budget_contract_${System.nanoTime()}"
        val jdbcUrl = "jdbc:h2:mem:$dbName;MODE=PostgreSQL;DB_CLOSE_DELAY=-1"
        return TestFixture(
            subject = KtormBudgetPort(
                jdbcUrl = jdbcUrl,
                tableName = "rs_budget_$dbName"
            )
        )
    }
}


