package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.adapter.infrastructure.SystemClockPort
import fuookami.ospf.framework.remote_solver.adapter.infrastructure.UUIDIdGeneratorPort
import fuookami.ospf.framework.remote_solver.adapter.ktorm.KtormDistributedLockPort
import fuookami.ospf.framework.remote_solver.port.DistributedLockPort

class KtormDistributedLockPortContractTest : DistributedLockPortContractTest() {
    override fun createFixture(): TestFixture<DistributedLockPort> {
        val dbName = "lock_contract_${System.nanoTime()}"
        val jdbcUrl = "jdbc:h2:mem:$dbName;MODE=PostgreSQL;DB_CLOSE_DELAY=-1"
        return TestFixture(
            subject = KtormDistributedLockPort(
                clock = SystemClockPort(),
                idGenerator = UUIDIdGeneratorPort(),
                jdbcUrl = jdbcUrl,
                tableName = "rs_lock_$dbName"
            )
        )
    }
}


