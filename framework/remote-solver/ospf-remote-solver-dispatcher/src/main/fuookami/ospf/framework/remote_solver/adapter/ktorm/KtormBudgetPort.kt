/**
 * 预算管理端口模块
 *
 * 本模块提供基于 Ktorm 的预算管理功能，
 * 用于远程求解器的成本控制和预算限制管理。
 *
 * Budget Management Port Module
 *
 * This module provides budget management functionality based on Ktorm,
 * used for cost control and budget limit management in the remote solver.
 */
package fuookami.ospf.framework.remote_solver.adapter.ktorm

import fuookami.ospf.framework.remote_solver.domain.BudgetSnapshot
import fuookami.ospf.framework.remote_solver.port.BudgetPort
import org.ktorm.database.Database
import org.ktorm.dsl.and
import org.ktorm.dsl.eq
import org.ktorm.dsl.from
import org.ktorm.dsl.greaterEq
import org.ktorm.dsl.insert
import org.ktorm.dsl.map
import org.ktorm.dsl.minus
import org.ktorm.dsl.notEq
import org.ktorm.dsl.plus
import org.ktorm.dsl.select
import org.ktorm.dsl.update
import org.ktorm.dsl.where
import org.ktorm.schema.double
import org.ktorm.schema.varchar
import java.util.concurrent.atomic.AtomicBoolean

/**
 * Ktorm 预算端口
 *
 * 基于 Ktorm 实现的预算管理端口，提供预算配置、预留、提交和退还功能。
 * 支持多作用域的预算管理，确保并发安全。
 *
 * 表结构：remote_solver_budget
 *
 * Ktorm Budget Port
 *
 * Budget management port implemented with Ktorm, providing budget configuration,
 * reservation, commit, and refund functionality. Supports multi-scope budget
 * management with concurrent safety.
 *
 * Table schema: remote_solver_budget
 *
 * @param jdbcUrl JDBC 连接 URL
 * @param username 数据库用户名，可选
 * @param password 数据库密码，可选
 * @param tableName 表名称，默认为 remote_solver_budget
 */
class KtormBudgetPort(
    private val jdbcUrl: String,
    private val username: String? = null,
    private val password: String? = null,
    tableName: String = "remote_solver_budget"
) : BudgetPort {
    private val resolvedTableName = normalizeTableName(tableName)
    private val table = BudgetTable(resolvedTableName)
    private val initialized = AtomicBoolean(false)
    private val database = Database.connect(
        url = jdbcUrl,
        user = username,
        password = password
    )

    /**
     * 配置预算
     *
     * 为指定作用域设置预算上限，如果已存在则更新，不存在则插入。
     *
     * @param scope 预算作用域标识
     * @param limit 预算上限金额
     *
     * Configures budget
     *
     * Sets the budget limit for the specified scope. Updates if exists,
     * inserts if not exists.
     *
     * @param scope Budget scope identifier
     * @param limit Budget limit amount
     */
    override suspend fun configureBudget(scope: String, limit: Double) {
        ensureSchema()
        val updated = database.update(table) {
            set(it.limitAmount, limit)
            where { it.scope eq scope }
        }
        if (updated > 0) {
            return
        }
        database.insert(table) {
            set(it.scope, scope)
            set(it.limitAmount, limit)
            set(it.reservedAmount, 0.0)
            set(it.consumedAmount, 0.0)
        }
    }

    /**
     * 获取预算快照
     *
     * 获取指定作用域的当前预算状态快照。
     *
     * @param scope 预算作用域标识
     * @return 预算快照，如果不存在则返回 null
     *
     * Gets budget snapshot
     *
     * Retrieves the current budget state snapshot for the specified scope.
     *
     * @param scope Budget scope identifier
     * @return Budget snapshot, or null if not found
     */
    override suspend fun snapshot(scope: String): BudgetSnapshot? =
        run {
            ensureSchema()
            database
            .from(table)
            .select(
                table.scope,
                table.limitAmount,
                table.reservedAmount,
                table.consumedAmount
            )
            .where { table.scope eq scope }
            .map { row ->
                BudgetSnapshot(
                    scope = row[table.scope].orEmpty(),
                    limit = row[table.limitAmount] ?: 0.0,
                    reserved = row[table.reservedAmount] ?: 0.0,
                    consumed = row[table.consumedAmount] ?: 0.0
                )
            }
            .firstOrNull()
        }

    /**
     * 预留预算
     *
     * 尝试为指定作用域预留预算金额，只有在可用余额充足时才能成功。
     *
     * @param scope 预算作用域标识
     * @param amount 要预留的金额
     * @return 是否成功预留
     *
     * Reserves budget
     *
     * Attempts to reserve budget amount for the specified scope.
     * Only succeeds when available balance is sufficient.
     *
     * @param scope Budget scope identifier
     * @param amount Amount to reserve
     * @return Whether reservation succeeded
     */
    override suspend fun reserve(scope: String, amount: Double): Boolean {
        if (amount < 0.0) {
            return false
        }
        ensureSchema()
        val updated = database.update(table) {
            set(it.reservedAmount, it.reservedAmount plus amount)
            where {
                (it.scope eq scope) and (
                    (it.limitAmount - it.reservedAmount - it.consumedAmount) greaterEq amount
                    )
            }
        }
        return updated > 0
    }

    /**
     * 提交预算
     *
     * 将预留的预算转换为已消费金额，或直接消费可用余额。
     *
     * @param scope 预算作用域标识
     * @param amount 要提交的金额
     * @return 是否成功提交
     *
     * Commits budget
     *
     * Converts reserved budget to consumed amount,
     * or directly consumes available balance.
     *
     * @param scope Budget scope identifier
     * @param amount Amount to commit
     * @return Whether commit succeeded
     */
    override suspend fun commit(scope: String, amount: Double): Boolean {
        if (amount < 0.0) {
            return false
        }
        ensureSchema()
        val reservedCommit = database.update(table) {
            set(it.reservedAmount, it.reservedAmount minus amount)
            set(it.consumedAmount, it.consumedAmount plus amount)
            where {
                (it.scope eq scope) and (it.reservedAmount greaterEq amount)
            }
        }
        if (reservedCommit > 0) {
            return true
        }
        val directCommit = database.update(table) {
            set(it.consumedAmount, it.consumedAmount plus amount)
            where {
                (it.scope eq scope) and (
                    (it.limitAmount - it.reservedAmount - it.consumedAmount) greaterEq amount
                    )
            }
        }
        return directCommit > 0
    }

    /**
     * 退还预算
     *
     * 将已消费的预算退还到可用余额。
     *
     * @param scope 预算作用域标识
     * @param amount 要退还的金额
     * @return 是否成功退还
     *
     * Refunds budget
     *
     * Refunds consumed budget back to available balance.
     *
     * @param scope Budget scope identifier
     * @param amount Amount to refund
     * @return Whether refund succeeded
     */
    override suspend fun refund(scope: String, amount: Double): Boolean {
        if (amount < 0.0) {
            return false
        }
        ensureSchema()
        val current = snapshot(scope) ?: return false
        val nextConsumed = (current.consumed - amount).coerceAtLeast(0.0)
        val updated = database.update(table) {
            set(it.consumedAmount, nextConsumed)
            where { it.scope eq scope }
        }
        return updated > 0
    }

    /**
     * 规范化表名称
     *
     * 验证并规范化表名称，确保符合数据库命名规范。
     *
     * @param raw 原始表名称
     * @return 规范化后的表名称
     * @throws IllegalArgumentException 如果表名称无效
     *
     * Normalizes table name
     *
     * Validates and normalizes the table name, ensuring it conforms
     * to database naming conventions.
     *
     * @param raw Raw table name
     * @return Normalized table name
     * @throws IllegalArgumentException If table name is invalid
     */
    private fun normalizeTableName(raw: String): String {
        val value = raw.trim()
        require(value.isNotEmpty()) { "tableName must not be blank" }
        require(value.matches(Regex("[A-Za-z0-9_]+"))) { "Invalid tableName: '$raw'" }
        return value
    }

    /**
     * 确保数据库表结构存在
     *
     * 检查并创建必要的数据库表结构，使用双重检查锁定确保线程安全。
     *
     * Ensures database schema exists
     *
     * Checks and creates necessary database table structures,
     * using double-checked locking for thread safety.
     */
    private fun ensureSchema() {
        if (initialized.get()) {
            return
        }
        synchronized(initialized) {
            if (initialized.get()) {
                return
            }
            database.useConnection { connection ->
                connection.createStatement().use { statement ->
                    statement.execute(
                        """
                        CREATE TABLE IF NOT EXISTS $resolvedTableName (
                            scope VARCHAR(256) PRIMARY KEY,
                            limit_amount DOUBLE PRECISION NOT NULL,
                            reserved_amount DOUBLE PRECISION NOT NULL,
                            consumed_amount DOUBLE PRECISION NOT NULL
                        )
                        """.trimIndent()
                    )
                }
            }
            initialized.set(true)
        }
    }

    /**
     * 预算表定义
     *
     * 定义存储预算信息的数据库表结构。
     *
     * Budget Table Definition
     *
     * Defines the database table structure for storing budget information.
     *
     * @param tableName 表名称
     */
    private class BudgetTable(tableName: String) : org.ktorm.schema.Table<Nothing>(tableName) {
        val scope = varchar("scope").primaryKey()
        val limitAmount = double("limit_amount")
        val reservedAmount = double("reserved_amount")
        val consumedAmount = double("consumed_amount")
    }
}