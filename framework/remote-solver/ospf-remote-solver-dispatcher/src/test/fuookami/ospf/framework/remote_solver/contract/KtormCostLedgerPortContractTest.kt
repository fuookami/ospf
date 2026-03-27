package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.adapter.ktorm.KtormCostLedgerPort
import fuookami.ospf.framework.remote_solver.port.CostLedgerPort

class KtormCostLedgerPortContractTest : CostLedgerPortContractTest() {
    override fun createFixture(): TestFixture<CostLedgerPort> {
        val dbName = "cost_contract_${System.nanoTime()}"
        val jdbcUrl = "jdbc:h2:mem:$dbName;MODE=PostgreSQL;DB_CLOSE_DELAY=-1"
        return TestFixture(
            subject = KtormCostLedgerPort(
                jdbcUrl = jdbcUrl,
                tableName = "rs_cost_$dbName"
            )
        )
    }
}


