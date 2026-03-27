/**
 * 内存预算端口适配器
 *
 * 提供基于内存的预算管理实现，用于测试和非持久化场景。
 * 支持预算配置、预留、提交和退款操作。
 *
 * In-memory budget port adapter.
 *
 * Provides memory-based budget management implementation for testing and non-persistent scenarios.
 * Supports budget configuration, reservation, commitment, and refund operations.
 */
package fuookami.ospf.framework.remote_solver.adapter.inmemory

import fuookami.ospf.framework.remote_solver.domain.BudgetSnapshot
import fuookami.ospf.framework.remote_solver.port.BudgetPort
import java.util.concurrent.ConcurrentHashMap

/**
 * 内存预算端口实现
 *
 * 使用内存存储预算快照，支持预算的生命周期管理。
 *
 * In-memory budget port implementation.
 *
 * Uses in-memory storage for budget snapshots, supporting budget lifecycle management.
 */
class InMemoryBudgetPort : BudgetPort {
    /**
     * 预算存储映射
     *
     * 键为预算范围标识，值为预算快照。
     *
     * Budget storage map.
     *
     * Keyed by budget scope identifier, valued by budget snapshot.
     */
    private val budgets = ConcurrentHashMap<String, BudgetSnapshot>()

    /**
     * 配置预算
     *
     * 为指定范围设置预算上限。如果预算已存在，仅更新上限值。
     *
     * Configures budget.
     *
     * Sets budget limit for the specified scope. If budget already exists, only updates the limit value.
     *
     * @param scope 预算范围标识
     *              Budget scope identifier
     * @param limit 预算上限
     *              Budget limit
     */
    override suspend fun configureBudget(scope: String, limit: Double) {
        synchronized(budgets) {
            val current = budgets[scope]
            budgets[scope] = if (current == null) {
                BudgetSnapshot(scope = scope, limit = limit, reserved = 0.0, consumed = 0.0)
            } else {
                current.copy(limit = limit)
            }
        }
    }

    /**
     * 获取预算快照
     *
     * 返回指定范围的当前预算状态。
     *
     * Gets budget snapshot.
     *
     * Returns current budget state for the specified scope.
     *
     * @param scope 预算范围标识
     *              Budget scope identifier
     * @return 预算快照，如果不存在则返回null
     *         Budget snapshot, returns null if not found
     */
    override suspend fun snapshot(scope: String): BudgetSnapshot? = budgets[scope]

    /**
     * 预留预算
     *
     * 从可用预算中预留指定金额，用于后续消费。
     * 如果可用余额不足，操作将失败。
     *
     * Reserves budget.
     *
     * Reserves specified amount from available budget for future consumption.
     * If available balance is insufficient, the operation will fail.
     *
     * @param scope 预算范围标识
     *              Budget scope identifier
     * @param amount 预留金额
     *               Amount to reserve
     * @return 是否成功预留
     *         Whether reservation was successful
     */
    override suspend fun reserve(scope: String, amount: Double): Boolean {
        if (amount < 0) {
            return false
        }
        synchronized(budgets) {
            val current = budgets[scope] ?: return false
            if (current.remaining < amount) {
                return false
            }
            budgets[scope] = current.copy(reserved = current.reserved + amount)
            return true
        }
    }

    /**
     * 提交预算消费
     *
     * 将预留或可用预算确认为实际消费。
     * 如果有足够的预留金额，将从中扣除；否则从可用余额中扣除。
     *
     * Commits budget consumption.
     *
     * Confirms reserved or available budget as actual consumption.
     * If there's sufficient reserved amount, it will be deducted from reservation; otherwise from available balance.
     *
     * @param scope 预算范围标识
     *              Budget scope identifier
     * @param amount 消费金额
     *               Consumption amount
     * @return 是否成功提交
     *         Whether commitment was successful
     */
    override suspend fun commit(scope: String, amount: Double): Boolean {
        if (amount < 0) {
            return false
        }
        synchronized(budgets) {
            val current = budgets[scope] ?: return false
            val next = if (current.reserved >= amount) {
                current.copy(
                    reserved = current.reserved - amount,
                    consumed = current.consumed + amount
                )
            } else {
                if (current.remaining < amount) {
                    return false
                }
                current.copy(consumed = current.consumed + amount)
            }
            budgets[scope] = next
            return true
        }
    }

    /**
     * 退款
     *
     * 将已消费的预算返还到可用余额中。
     *
     * Refunds budget.
     *
     * Returns consumed budget back to available balance.
     *
     * @param scope 预算范围标识
     *              Budget scope identifier
     * @param amount 退款金额
     *               Refund amount
     * @return 是否成功退款
     *         Whether refund was successful
     */
    override suspend fun refund(scope: String, amount: Double): Boolean {
        if (amount < 0) {
            return false
        }
        synchronized(budgets) {
            val current = budgets[scope] ?: return false
            val consumed = (current.consumed - amount).coerceAtLeast(0.0)
            budgets[scope] = current.copy(consumed = consumed)
            return true
        }
    }
}