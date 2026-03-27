package fuookami.ospf.framework.remote_solver.adapter.ktorm

import fuookami.ospf.framework.remote_solver.application.SchedulerHotReloadAuditRecord
import fuookami.ospf.framework.remote_solver.application.SchedulerRuntimeConfig
import fuookami.ospf.framework.remote_solver.contract.runSuspend
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

class KtormSchedulerConfigAuditPortTest {
    @Test
    fun appendListAndSnapshotShouldWork() {
        val dbName = "scheduler_audit_${System.nanoTime()}"
        val jdbcUrl = "jdbc:h2:mem:$dbName;MODE=PostgreSQL;DB_CLOSE_DELAY=-1"
        val port = KtormSchedulerConfigAuditPort(
            jdbcUrl = jdbcUrl,
            auditTableName = "rs_scheduler_audit_$dbName",
            snapshotTableName = "rs_scheduler_snapshot_$dbName"
        )

        runSuspend {
            port.append(
                SchedulerHotReloadAuditRecord(
                    version = "v1",
                    previousVersion = "v0",
                    operator = "tester",
                    effectiveAtEpochMs = 1000L,
                    changeSet = mapOf("scheduler.simple-task-quantum-ms" to "3000"),
                    rollbackFromVersion = null
                )
            )
            port.append(
                SchedulerHotReloadAuditRecord(
                    version = "v2",
                    previousVersion = "v1",
                    operator = "tester",
                    effectiveAtEpochMs = 2000L,
                    changeSet = mapOf("scheduler.complex-urgency-weight" to "0.6"),
                    rollbackFromVersion = "v1"
                )
            )

            val audits = port.list(limit = 10)
            assertEquals(2, audits.size)
            assertEquals("v1", audits[0].version)
            assertEquals("v2", audits[1].version)
            assertEquals("v1", audits[1].rollbackFromVersion)
            assertEquals("0.6", audits[1].changeSet["scheduler.complex-urgency-weight"])

            val snapshot = SchedulerRuntimeConfig(
                simpleTaskQuantumMs = 2100L,
                complexTaskQuantumMs = 4200L,
                complexTaskQuantumMinMs = 2000L,
                complexTaskQuantumMaxMs = 10000L,
                complexSolveEstimateMs = 12000L,
                complexCheckpointEstimateMs = 800L,
                complexQuantumAlpha = 0.7,
                complexQuantumBeta = 0.5,
                complexQuantumPricePenalty = 0.1,
                complexUrgencyWeight = 0.4,
                complexWaitingAgeWeight = 0.3,
                complexProgressNeedWeight = 0.2,
                complexCostSensitivityWeight = 0.1
            )
            port.saveSnapshot("v2", snapshot)
            val recovered = port.getSnapshot("v2")
            assertNotNull(recovered)
            assertEquals(2100L, recovered.simpleTaskQuantumMs)
            assertEquals(0.4, recovered.complexUrgencyWeight)
        }
    }

    @Test
    fun listShouldRespectLimit() {
        val dbName = "scheduler_audit_limit_${System.nanoTime()}"
        val jdbcUrl = "jdbc:h2:mem:$dbName;MODE=PostgreSQL;DB_CLOSE_DELAY=-1"
        val port = KtormSchedulerConfigAuditPort(
            jdbcUrl = jdbcUrl,
            auditTableName = "rs_scheduler_audit_$dbName",
            snapshotTableName = "rs_scheduler_snapshot_$dbName"
        )
        runSuspend {
            (1..5).forEach { idx ->
                port.append(
                    SchedulerHotReloadAuditRecord(
                        version = "v$idx",
                        previousVersion = "v${idx - 1}",
                        operator = "tester",
                        effectiveAtEpochMs = idx.toLong(),
                        changeSet = emptyMap(),
                        rollbackFromVersion = null
                    )
                )
            }
            val limited = port.list(limit = 2)
            assertEquals(2, limited.size)
            assertEquals("v4", limited[0].version)
            assertEquals("v5", limited[1].version)
            assertTrue(limited[0].effectiveAtEpochMs < limited[1].effectiveAtEpochMs)
        }
    }
}

