package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.adapter.ktorm.KtormNodeStatePort
import fuookami.ospf.framework.remote_solver.port.NodeStatePort

class KtormNodeStatePortContractTest : NodeStatePortContractTest() {
    override fun createFixture(): TestFixture<NodeStatePort> {
        val dbName = "node_contract_${System.nanoTime()}"
        val jdbcUrl = "jdbc:h2:mem:$dbName;MODE=PostgreSQL;DB_CLOSE_DELAY=-1"
        return TestFixture(
            subject = KtormNodeStatePort(
                jdbcUrl = jdbcUrl,
                tableName = "rs_node_$dbName"
            )
        )
    }
}


