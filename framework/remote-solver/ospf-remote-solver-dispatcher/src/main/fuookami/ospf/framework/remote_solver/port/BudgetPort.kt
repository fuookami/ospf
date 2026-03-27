/*
 * 预算端口接口
 *
 * Budget Port Interface
 *
 * 该接口定义了预算管理的核心抽象，支持预算配置、预留、提交和退款操作。
 * This interface defines the core abstraction for budget management,
 * supporting budget configuration, reservation, commitment, and refund operations.
 *
 * 预算端口用于控制计算资源消耗，防止超出预算限制。
 * The budget port is used to control computational resource consumption
 * and prevent exceeding budget limits.
 *
 * 典型使用流程：
 * Typical usage flow:
 * 1. configureBudget() - 配置预算范围和限额 / Configure budget scope and limit
 * 2. reserve() - 预留预算金额 / Reserve budget amount
 * 3. commit() - 提交实际消耗 / Commit actual consumption
 * 4. refund() - 退款未使用的预留 / Refund unused reservation
 */
package fuookami.ospf.framework.remote_solver.port

import fuookami.ospf.framework.remote_solver.domain.BudgetSnapshot

/**
 * 预算端口接口
 *
 * Budget Port Interface
 *
 * 提供预算管理功能的端口接口，支持预算的配置、查询和操作。
 * Port interface providing budget management capabilities including configuration, query, and operations.
 */
interface BudgetPort {
    /**
     * 配置预算
     *
     * Configures a budget with specified scope and limit.
     *
     * 为指定的范围配置预算限额。如果预算已存在，将更新限额。
     * Configures a budget limit for the specified scope. If budget exists, updates the limit.
     *
     * @param scope 预算范围标识符（如租户 ID 或项目 ID）
     *              Budget scope identifier (e.g., tenant ID or project ID)
     * @param limit 预算限额（非负数值）
     *              Budget limit (non-negative value)
     */
    suspend fun configureBudget(scope: String, limit: Double)

    /**
     * 获取预算快照
     *
     * Gets a snapshot of the current budget state.
     *
     * @param scope 预算范围标识符 / Budget scope identifier
     * @return 预算快照，如不存在则返回 null
     *         Budget snapshot, or null if not found
     */
    suspend fun snapshot(scope: String): BudgetSnapshot?

    /**
     * 预留预算
     *
     * Reserves budget amount for a scope.
     *
     * 尝试为指定范围预留预算金额。如果可用余额不足，操作将失败。
     * Attempts to reserve budget amount for the specified scope.
     * Operation fails if available balance is insufficient.
     *
     * @param scope 预算范围标识符 / Budget scope identifier
     * @param amount 要预留的金额 / Amount to reserve
     * @return 预留成功返回 true，余额不足返回 false
     *         true if reservation succeeded, false if insufficient balance
     */
    suspend fun reserve(scope: String, amount: Double): Boolean

    /**
     * 提交预算消耗
     *
     * Commits budget consumption.
     *
     * 将预留的预算金额确认为实际消耗。通常会从预留金额中扣除。
     * Confirms the reserved budget amount as actual consumption.
     * Typically deducted from the reserved amount.
     *
     * @param scope 预算范围标识符 / Budget scope identifier
     * @param amount 实际消耗金额 / Actual consumption amount
     * @return 提交成功返回 true，失败返回 false
     *         true if commit succeeded, false otherwise
     */
    suspend fun commit(scope: String, amount: Double): Boolean

    /**
     * 退款预留预算
     *
     * Refunds reserved budget.
     *
     * 将未使用的预留金额返还到可用余额。
     * Returns unused reserved amount back to available balance.
     *
     * @param scope 预算范围标识符 / Budget scope identifier
     * @param amount 要退还的金额 / Amount to refund
     * @return 退款成功返回 true，失败返回 false
     *         true if refund succeeded, false otherwise
     */
    suspend fun refund(scope: String, amount: Double): Boolean
}