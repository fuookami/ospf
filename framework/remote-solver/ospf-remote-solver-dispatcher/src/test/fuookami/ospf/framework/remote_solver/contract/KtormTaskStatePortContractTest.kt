package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.adapter.ktorm.KtormTaskStatePort
import fuookami.ospf.framework.remote_solver.port.TaskStatePort

class KtormTaskStatePortContractTest : TaskStatePortContractTest() {
    override fun createFixture(): TestFixture<TaskStatePort> {
        val dbName = "task_contract_${System.nanoTime()}"
        val jdbcUrl = "jdbc:h2:mem:$dbName;MODE=PostgreSQL;DB_CLOSE_DELAY=-1"
        return TestFixture(
            subject = KtormTaskStatePort(
                jdbcUrl = jdbcUrl,
                taskTableName = "rs_task_$dbName",
                sliceTableName = "rs_slice_$dbName"
            )
        )
    }
}


