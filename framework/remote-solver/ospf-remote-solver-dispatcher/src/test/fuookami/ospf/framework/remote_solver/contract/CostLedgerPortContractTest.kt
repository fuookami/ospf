package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.domain.CostRecord
import fuookami.ospf.framework.remote_solver.port.CostLedgerPort
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

abstract class CostLedgerPortContractTest {
    protected abstract fun createFixture(): TestFixture<CostLedgerPort>

    @Test
    fun appendAndListByTaskShouldReturnSortedRecords() {
        val fixture = createFixture()
        try {
            runSuspend {
                fixture.subject.append(
                    sampleRecord(
                        taskId = "task-a",
                        sliceId = "slice-2",
                        createdAtEpochMs = 2000L,
                        totalCost = 4.0
                    )
                )
                fixture.subject.append(
                    sampleRecord(
                        taskId = "task-a",
                        sliceId = "slice-1",
                        createdAtEpochMs = 1000L,
                        totalCost = 3.0
                    )
                )
                fixture.subject.append(
                    sampleRecord(
                        taskId = "task-b",
                        sliceId = "slice-1",
                        createdAtEpochMs = 1500L,
                        totalCost = 2.0
                    )
                )

                val taskRecords = fixture.subject.listByTask("task-a")
                assertEquals(2, taskRecords.size)
                assertEquals("slice-1", taskRecords[0].sliceId)
                assertEquals("slice-2", taskRecords[1].sliceId)
            }
        } finally {
            fixture.close()
        }
    }

    @Test
    fun summarizeByTaskAndBudgetScopeShouldAccumulateCost() {
        val fixture = createFixture()
        try {
            runSuspend {
                fixture.subject.append(
                    sampleRecord(
                        taskId = "task-c",
                        budgetScope = "scope-c",
                        sliceId = "slice-1",
                        createdAtEpochMs = 1000L,
                        runtimeMs = 2000L,
                        billedSeconds = 2L,
                        licenseCost = 0.5,
                        totalCost = 2.5
                    )
                )
                fixture.subject.append(
                    sampleRecord(
                        taskId = "task-c",
                        budgetScope = "scope-c",
                        sliceId = "slice-2",
                        createdAtEpochMs = 2000L,
                        runtimeMs = 3000L,
                        billedSeconds = 3L,
                        licenseCost = 0.5,
                        totalCost = 3.5
                    )
                )
                fixture.subject.append(
                    sampleRecord(
                        taskId = "task-d",
                        budgetScope = "scope-c",
                        sliceId = "slice-1",
                        createdAtEpochMs = 3000L,
                        runtimeMs = 1000L,
                        billedSeconds = 1L,
                        licenseCost = 0.2,
                        totalCost = 1.2
                    )
                )

                val byTask = fixture.subject.summarizeByTask("task-c")
                assertEquals(2, byTask.recordCount)
                assertEquals(6.0, byTask.totalCost)
                assertEquals(5000L, byTask.totalRuntimeMs)

                val byScope = fixture.subject.summarizeByBudgetScope("scope-c")
                assertEquals(3, byScope.recordCount)
                assertEquals(6L, byScope.totalBilledSeconds)
                assertEquals(7.2, byScope.totalCost)
            }
        } finally {
            fixture.close()
        }
    }

    @Test
    fun emptySummaryShouldReturnZeroValues() {
        val fixture = createFixture()
        try {
            runSuspend {
                val summary = fixture.subject.summarizeByTask("missing-task")
                assertEquals(0, summary.recordCount)
                assertEquals(0L, summary.totalRuntimeMs)
                assertEquals(0.0, summary.totalCost)
            }
        } finally {
            fixture.close()
        }
    }

    private fun sampleRecord(
        taskId: String,
        budgetScope: String = "scope-default",
        sliceId: String,
        createdAtEpochMs: Long,
        runtimeMs: Long = 1000L,
        billedSeconds: Long = 1L,
        licenseCost: Double = 0.0,
        totalCost: Double = 1.0
    ): CostRecord =
        CostRecord(
            taskId = taskId,
            budgetScope = budgetScope,
            sliceId = sliceId,
            nodeId = "node-a",
            runtimeMs = runtimeMs,
            billedSeconds = billedSeconds,
            pricePerSecond = 1.0,
            licenseCost = licenseCost,
            totalCost = totalCost,
            createdAtEpochMs = createdAtEpochMs
        )
}
