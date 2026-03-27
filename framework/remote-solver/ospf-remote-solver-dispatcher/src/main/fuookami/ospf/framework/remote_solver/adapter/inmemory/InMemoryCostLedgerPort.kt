/**
 * 内存成本账本端口适配器
 *
 * 提供基于内存的成本记录和汇总实现，用于测试和非持久化场景。
 * 支持按任务和预算范围查询成本记录及汇总统计。
 *
 * In-memory cost ledger port adapter.
 *
 * Provides memory-based cost recording and summarization implementation for testing and non-persistent scenarios.
 * Supports querying cost records and summary statistics by task and budget scope.
 */
package fuookami.ospf.framework.remote_solver.adapter.inmemory

import fuookami.ospf.framework.remote_solver.domain.CostRecord
import fuookami.ospf.framework.remote_solver.domain.CostSummary
import fuookami.ospf.framework.remote_solver.port.CostLedgerPort
import fuookami.ospf.framework.remote_solver.protocol.domain.BudgetScopeId
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskId
import java.util.concurrent.CopyOnWriteArrayList

/**
 * 内存成本账本端口实现
 *
 * 使用内存存储成本记录，支持按不同维度进行查询和汇总。
 *
 * In-memory cost ledger port implementation.
 *
 * Uses in-memory storage for cost records, supporting queries and summarization by different dimensions.
 */
class InMemoryCostLedgerPort : CostLedgerPort {
    /**
     * 成本记录列表
     *
     * 存储所有成本记录。
     *
     * Cost record list.
     *
     * Stores all cost records.
     */
    private val records = CopyOnWriteArrayList<CostRecord>()

    /**
     * 追加成本记录
     *
     * 将新的成本记录添加到账本中。
     *
     * Appends cost record.
     *
     * Adds a new cost record to the ledger.
     *
     * @param record 成本记录
     *               Cost record
     */
    override suspend fun append(record: CostRecord) {
        records.add(record)
    }

    /**
     * 按任务ID列出成本记录
     *
     * 返回指定任务的所有成本记录，按创建时间升序排列。
     *
     * Lists cost records by task ID.
     *
     * Returns all cost records for the specified task, sorted by creation time in ascending order.
     *
     * @param taskId 任务ID
     *               Task ID
     * @return 成本记录列表
     *         Cost record list
     */
    override suspend fun listByTask(taskId: TaskId): List<CostRecord> =
        records
            .asSequence()
            .filter { it.taskId == taskId.value }
            .sortedBy { it.createdAtEpochMs }
            .toList()

    /**
     * 按预算范围列出成本记录
     *
     * 返回指定预算范围的所有成本记录，按创建时间升序排列。
     *
     * Lists cost records by budget scope.
     *
     * Returns all cost records for the specified budget scope, sorted by creation time in ascending order.
     *
     * @param scope 预算范围标识
     *              Budget scope identifier
     * @return 成本记录列表
     *         Cost record list
     */
    override suspend fun listByBudgetScope(scope: BudgetScopeId): List<CostRecord> =
        records
            .asSequence()
            .filter { it.budgetScope == scope.value }
            .sortedBy { it.createdAtEpochMs }
            .toList()

    /**
     * 按任务ID汇总成本
     *
     * 计算指定任务的成本汇总统计。
     *
     * Summarizes cost by task ID.
     *
     * Calculates cost summary statistics for the specified task.
     *
     * @param taskId 任务ID
     *               Task ID
     * @return 成本汇总
     *         Cost summary
     */
    override suspend fun summarizeByTask(taskId: TaskId): CostSummary =
        summarize(listByTask(taskId))

    /**
     * 按预算范围汇总成本
     *
     * 计算指定预算范围的成本汇总统计。
     *
     * Summarizes cost by budget scope.
     *
     * Calculates cost summary statistics for the specified budget scope.
     *
     * @param scope 预算范围标识
     *              Budget scope identifier
     * @return 成本汇总
     *         Cost summary
     */
    override suspend fun summarizeByBudgetScope(scope: BudgetScopeId): CostSummary =
        summarize(listByBudgetScope(scope))

    /**
     * 计算成本汇总统计
     *
     * 根据成本记录列表计算总运行时间、计费秒数、许可证成本和总成本。
     *
     * Calculates cost summary statistics.
     *
     * Calculates total runtime, billed seconds, license cost, and total cost from cost record list.
     *
     * @param records 成本记录列表
     *                Cost record list
     * @return 成本汇总，如果记录为空则返回空汇总
     *         Cost summary, returns empty summary if records are empty
     */
    private fun summarize(records: List<CostRecord>): CostSummary {
        if (records.isEmpty()) {
            return CostSummary.Empty
        }
        return CostSummary(
            recordCount = records.size,
            totalRuntimeMs = records.sumOf { it.runtimeMs },
            totalBilledSeconds = records.sumOf { it.billedSeconds },
            totalLicenseCost = records.sumOf { it.licenseCost },
            totalCost = records.sumOf { it.totalCost }
        )
    }
}
