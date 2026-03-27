/*
 * 成本账本端口接口
 *
 * Cost Ledger Port Interface
 *
 * 该接口定义了成本记录和汇总的核心抽象，支持按任务和预算范围追踪成本。
 * This interface defines the core abstraction for cost recording and summarization,
 * supporting cost tracking by task and budget scope.
 *
 * 成本账本用于记录和分析计算资源的实际消耗成本。
 * The cost ledger is used to record and analyze actual consumption costs of computational resources.
 *
 * 功能特性：
 * Features:
 * - 追踪每个任务的成本明细 / Track cost details for each task
 * - 按预算范围汇总成本 / Summarize costs by budget scope
 * - 支持成本分析和报告 / Support cost analysis and reporting
 */
package fuookami.ospf.framework.remote_solver.port

import fuookami.ospf.framework.remote_solver.domain.CostRecord
import fuookami.ospf.framework.remote_solver.domain.CostSummary
import fuookami.ospf.framework.remote_solver.protocol.domain.BudgetScopeId
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskId

/**
 * 成本账本端口接口
 *
 * Cost Ledger Port Interface
 *
 * 提供成本记录和汇总功能的端口接口。
 * Port interface providing cost recording and summarization capabilities.
 */
interface CostLedgerPort {
    /**
     * 追加成本记录
     *
     * Appends a cost record to the ledger.
     *
     * 将一条成本记录添加到账本中。
     * Adds a cost record entry to the ledger.
     *
     * @param record 成本记录 / Cost record to append
     */
    suspend fun append(record: CostRecord)

    /**
     * 按任务 ID 查询成本记录
     *
     * Lists cost records by task ID.
     *
     * 检索指定任务的所有成本记录。
     * Retrieves all cost records for the specified task.
     *
     * @param taskId 任务唯一标识符 / Unique task identifier
     * @return 成本记录列表 / List of cost records
     */
    suspend fun listByTask(taskId: String): List<CostRecord> = listByTask(TaskId.of(taskId))

    suspend fun listByTask(taskId: TaskId): List<CostRecord>

    /**
     * 按预算范围查询成本记录
     *
     * Lists cost records by budget scope.
     *
     * 检索指定预算范围内的所有成本记录。
     * Retrieves all cost records for the specified budget scope.
     *
     * @param scope 预算范围标识符 / Budget scope identifier
     * @return 成本记录列表 / List of cost records
     */
    suspend fun listByBudgetScope(scope: String): List<CostRecord> = listByBudgetScope(BudgetScopeId.of(scope))

    suspend fun listByBudgetScope(scope: BudgetScopeId): List<CostRecord>

    /**
     * 按任务 ID 汇总成本
     *
     * Summarizes costs by task ID.
     *
     * 计算指定任务的成本汇总统计。
     * Calculates cost summary statistics for the specified task.
     *
     * @param taskId 任务唯一标识符 / Unique task identifier
     * @return 成本汇总对象 / Cost summary object
     */
    suspend fun summarizeByTask(taskId: String): CostSummary = summarizeByTask(TaskId.of(taskId))

    suspend fun summarizeByTask(taskId: TaskId): CostSummary

    /**
     * 按预算范围汇总成本
     *
     * Summarizes costs by budget scope.
     *
     * 计算指定预算范围内的成本汇总统计。
     * Calculates cost summary statistics for the specified budget scope.
     *
     * @param scope 预算范围标识符 / Budget scope identifier
     * @return 成本汇总对象 / Cost summary object
     */
    suspend fun summarizeByBudgetScope(scope: String): CostSummary = summarizeByBudgetScope(BudgetScopeId.of(scope))

    suspend fun summarizeByBudgetScope(scope: BudgetScopeId): CostSummary
}
