/*
 * Infrastructure Models
 * 基础设施模型
 *
 * This file defines the infrastructure-level data structures for the Remote Solver Dispatcher.
 * 本文件定义了远程求解器调度器的基础设施级数据结构。
 *
 * These models support:
 * 这些模型支持：
 * - Distributed locking for coordination
 * - 分布式锁用于协调
 * - Budget management and tracking
 * - 预算管理和追踪
 * - Cost recording and summary
 * - 成本记录和汇总
 */
package fuookami.ospf.framework.remote_solver.domain

/**
 * Lock Lease
 * 锁租约
 *
 * Represents a distributed lock lease for coordination between multiple dispatcher instances.
 * 表示分布式锁租约，用于多个调度器实例之间的协调。
 *
 * @param key The lock key/resource being locked.
 *            被锁定的锁键/资源。
 * @param leaseId Unique identifier for this lease.
 *                此租约的唯一标识符。
 * @param owner The owner (dispatcher instance) holding the lock.
 *              持有锁的所有者（调度器实例）。
 * @param expiresAtEpochMs Expiration timestamp in epoch milliseconds.
 *                          过期时间戳（epoch毫秒）。
 */
data class LockLease(
    val key: String,
    val leaseId: String,
    val owner: String,
    val expiresAtEpochMs: Long
)

/**
 * Budget Snapshot
 * 预算快照
 *
 * A snapshot of budget state at a point in time.
 * 某时刻预算状态的快照。
 *
 * Used for tracking budget usage and preventing overspending.
 * 用于追踪预算使用和防止超支。
 *
 * @param scope The budget scope (e.g., tenant, project).
 *              预算范围（如租户、项目）。
 * @param limit The budget limit (maximum allowed spending).
 *              预算限制（允许的最大支出）。
 * @param reserved Amount reserved for pending tasks.
 *                  为待处理任务预留的金额。
 * @param consumed Amount already consumed by completed tasks.
 *                  已完成任务已消耗的金额。
 */
data class BudgetSnapshot(
    val scope: String,
    val limit: Double,
    val reserved: Double,
    val consumed: Double
) {
    /**
     * Remaining budget available for new reservations.
     * 可用于新预留的剩余预算。
     */
    val remaining: Double
        get() = limit - reserved - consumed
}

/**
 * Cost Record
 * 成本记录
 *
 * A detailed record of cost incurred by a task slice execution.
 * 任务Slice执行产生的成本的详细记录。
 *
 * @param taskId The task identifier.
 *               任务标识符。
 * @param budgetScope The budget scope this cost belongs to.
 *                     此成本所属的预算范围。
 * @param sliceId The slice identifier.
 *                Slice标识符。
 * @param nodeId The solver node that executed this slice.
 *               执行此Slice的求解器节点。
 * @param runtimeMs Actual runtime in milliseconds.
 *                   实际运行时间（毫秒）。
 * @param billedSeconds Billed time in seconds (may differ from runtime due to billing units).
 *                      计费时间（秒，可能因计费单位与运行时间不同）。
 * @param pricePerSecond Price per second charged by the node.
 *                        节点收取的每秒价格。
 * @param licenseCost Additional license cost for this slice.
 *                    此Slice的额外许可证成本。
 * @param totalCost Total cost (runtime cost + license cost).
 *                   总成本（运行时间成本 + 许可证成本）。
 * @param createdAtEpochMs Creation timestamp in epoch milliseconds.
 *                          创建时间戳（epoch毫秒）。
 */
data class CostRecord(
    val taskId: String,
    val budgetScope: String,
    val sliceId: String,
    val nodeId: String,
    val runtimeMs: Long,
    val billedSeconds: Long,
    val pricePerSecond: Double,
    val licenseCost: Double,
    val totalCost: Double,
    val createdAtEpochMs: Long
)

/**
 * Cost Summary
 * 成本汇总
 *
 * Aggregated summary of cost records.
 * 成本记录的聚合汇总。
 *
 * Used for reporting and analysis of overall spending.
 * 用于报告和分析整体支出。
 *
 * @param recordCount Number of cost records included in this summary.
 *                    此汇总中包含的成本记录数。
 * @param totalRuntimeMs Total runtime across all records in milliseconds.
 *                       所有记录的总运行时间（毫秒）。
 * @param totalBilledSeconds Total billed time across all records in seconds.
 *                            所有记录的总计费时间（秒）。
 * @param totalLicenseCost Total license cost across all records.
 *                          所有记录的总许可证成本。
 * @param totalCost Total cost across all records.
 *                  所有记录的总成本。
 */
data class CostSummary(
    val recordCount: Int,
    val totalRuntimeMs: Long,
    val totalBilledSeconds: Long,
    val totalLicenseCost: Double,
    val totalCost: Double
) {
    companion object {
        /**
         * Empty cost summary with zero values.
         * 零值的空成本汇总。
         */
        val Empty = CostSummary(
            recordCount = 0,
            totalRuntimeMs = 0L,
            totalBilledSeconds = 0L,
            totalLicenseCost = 0.0,
            totalCost = 0.0
        )
    }
}