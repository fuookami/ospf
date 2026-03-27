/**
 * 成本账本端口模块
 *
 * 本模块提供基于 Ktorm 的成本账本功能，
 * 用于记录和汇总远程求解器的成本消费记录。
 *
 * Cost Ledger Port Module
 *
 * This module provides cost ledger functionality based on Ktorm,
 * used for recording and summarizing cost consumption records
 * in the remote solver.
 */
package fuookami.ospf.framework.remote_solver.adapter.ktorm

import fuookami.ospf.framework.remote_solver.domain.CostRecord
import fuookami.ospf.framework.remote_solver.domain.CostSummary
import fuookami.ospf.framework.remote_solver.port.CostLedgerPort
import fuookami.ospf.framework.remote_solver.protocol.domain.BudgetScopeId
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskId
import org.ktorm.database.Database
import org.ktorm.dsl.asc
import org.ktorm.dsl.eq
import org.ktorm.dsl.from
import org.ktorm.dsl.insert
import org.ktorm.dsl.map
import org.ktorm.dsl.orderBy
import org.ktorm.dsl.select
import org.ktorm.dsl.where
import org.ktorm.schema.double
import org.ktorm.schema.long
import org.ktorm.schema.varchar
import java.util.concurrent.atomic.AtomicBoolean

/**
 * Ktorm 成本账本端口
 *
 * 基于 Ktorm 实现的成本账本端口，提供成本记录追加、
 * 查询和汇总功能。支持按任务或预算作用域进行成本统计。
 *
 * 表结构：remote_solver_cost_ledger
 * 索引：
 * - idx_remote_solver_cost_ledger_task: 任务 ID 和创建时间索引
 * - idx_remote_solver_cost_ledger_scope: 预算作用域和创建时间索引
 *
 * Ktorm Cost Ledger Port
 *
 * Cost ledger port implemented with Ktorm, providing cost record appending,
 * querying, and summarization functionality. Supports cost statistics
 * by task or budget scope.
 *
 * Table schema: remote_solver_cost_ledger
 * Indexes:
 * - idx_remote_solver_cost_ledger_task: Task ID and creation time index
 * - idx_remote_solver_cost_ledger_scope: Budget scope and creation time index
 *
 * @param jdbcUrl JDBC 连接 URL
 * @param username 数据库用户名，可选
 * @param password 数据库密码，可选
 * @param tableName 表名称，默认为 remote_solver_cost_ledger
 */
class KtormCostLedgerPort(
    private val jdbcUrl: String,
    private val username: String? = null,
    private val password: String? = null,
    tableName: String = "remote_solver_cost_ledger"
) : CostLedgerPort {
    private val resolvedTableName = normalizeTableName(tableName)
    private val table = CostLedgerTable(resolvedTableName)
    private val initialized = AtomicBoolean(false)
    private val database = Database.connect(
        url = jdbcUrl,
        user = username,
        password = password
    )

    /**
     * 添加成本记录
     *
     * 将成本消费记录追加到账本中。
     *
     * @param record 成本消费记录
     *
     * Appends cost record
     *
     * Appends a cost consumption record to the ledger.
     *
     * @param record Cost consumption record
     */
    override suspend fun append(record: CostRecord) {
        ensureSchema()
        database.insert(table) {
            set(it.taskId, record.taskId)
            set(it.budgetScope, record.budgetScope)
            set(it.sliceId, record.sliceId)
            set(it.nodeId, record.nodeId)
            set(it.runtimeMs, record.runtimeMs)
            set(it.billedSeconds, record.billedSeconds)
            set(it.pricePerSecond, record.pricePerSecond)
            set(it.licenseCost, record.licenseCost)
            set(it.totalCost, record.totalCost)
            set(it.createdAtEpochMs, record.createdAtEpochMs)
        }
    }

    /**
     * 查询任务成本记录列表
     *
     * 获取指定任务的所有成本消费记录。
     *
     * @param taskId 任务 ID
     * @return 成本记录列表
     *
     * Lists cost records by task
     *
     * Retrieves all cost consumption records for the specified task.
     *
     * @param taskId Task ID
     * @return List of cost records
     */
    override suspend fun listByTask(taskId: TaskId): List<CostRecord> =
        queryList(filterByTaskId = taskId.value, filterByScope = null)

    /**
     * 查询预算作用域成本记录列表
     *
     * 获取指定预算作用域的所有成本消费记录。
     *
     * @param scope 预算作用域标识
     * @return 成本记录列表
     *
     * Lists cost records by budget scope
     *
     * Retrieves all cost consumption records for the specified budget scope.
     *
     * @param scope Budget scope identifier
     * @return List of cost records
     */
    override suspend fun listByBudgetScope(scope: BudgetScopeId): List<CostRecord> =
        queryList(filterByTaskId = null, filterByScope = scope.value)

    /**
     * 汇总任务成本
     *
     * 计算指定任务的成本消费汇总统计。
     *
     * @param taskId 任务 ID
     * @return 成本汇总统计
     *
     * Summarizes cost by task
     *
     * Calculates cost consumption summary statistics for the specified task.
     *
     * @param taskId Task ID
     * @return Cost summary statistics
     */
    override suspend fun summarizeByTask(taskId: TaskId): CostSummary =
        summarize(queryList(filterByTaskId = taskId.value, filterByScope = null))

    /**
     * 汇总预算作用域成本
     *
     * 计算指定预算作用域的成本消费汇总统计。
     *
     * @param scope 预算作用域标识
     * @return 成本汇总统计
     *
     * Summarizes cost by budget scope
     *
     * Calculates cost consumption summary statistics for the specified budget scope.
     *
     * @param scope Budget scope identifier
     * @return Cost summary statistics
     */
    override suspend fun summarizeByBudgetScope(scope: BudgetScopeId): CostSummary =
        summarize(queryList(filterByTaskId = null, filterByScope = scope.value))

    /**
     * 查询成本记录列表
     *
     * 根据过滤条件查询成本消费记录。
     *
     * @param filterByTaskId 任务 ID 过滤条件，可选
     * @param filterByScope 预算作用域过滤条件，可选
     * @return 成本记录列表
     *
     * Queries cost record list
     *
     * Queries cost consumption records based on filter conditions.
     *
     * @param filterByTaskId Task ID filter condition, optional
     * @param filterByScope Budget scope filter condition, optional
     * @return List of cost records
     */
    private fun queryList(filterByTaskId: String?, filterByScope: String?): List<CostRecord> {
        ensureSchema()
        var query = database
            .from(table)
            .select(
                table.taskId,
                table.budgetScope,
                table.sliceId,
                table.nodeId,
                table.runtimeMs,
                table.billedSeconds,
                table.pricePerSecond,
                table.licenseCost,
                table.totalCost,
                table.createdAtEpochMs
            )
        if (filterByTaskId != null) {
            query = query.where { table.taskId eq filterByTaskId }
        }
        if (filterByScope != null) {
            query = query.where { table.budgetScope eq filterByScope }
        }
        return query
            .orderBy(table.createdAtEpochMs.asc())
            .map { row ->
                CostRecord(
                    taskId = row[table.taskId].orEmpty(),
                    budgetScope = row[table.budgetScope].orEmpty(),
                    sliceId = row[table.sliceId].orEmpty(),
                    nodeId = row[table.nodeId].orEmpty(),
                    runtimeMs = row[table.runtimeMs] ?: 0L,
                    billedSeconds = row[table.billedSeconds] ?: 0L,
                    pricePerSecond = row[table.pricePerSecond] ?: 0.0,
                    licenseCost = row[table.licenseCost] ?: 0.0,
                    totalCost = row[table.totalCost] ?: 0.0,
                    createdAtEpochMs = row[table.createdAtEpochMs] ?: 0L
                )
            }
    }

    /**
     * 汇总成本记录
     *
     * 计算成本记录列表的汇总统计信息。
     *
     * @param records 成本记录列表
     * @return 成本汇总统计
     *
     * Summarizes cost records
     *
     * Calculates summary statistics for a list of cost records.
     *
     * @param records List of cost records
     * @return Cost summary statistics
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
     * 检查并创建必要的数据库表结构和索引。
     *
     * Ensures database schema exists
     *
     * Checks and creates necessary database table structures and indexes.
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
                            id BIGINT GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY,
                            task_id VARCHAR(256) NOT NULL,
                            budget_scope VARCHAR(256) NOT NULL,
                            slice_id VARCHAR(256) NOT NULL,
                            node_id VARCHAR(256) NOT NULL,
                            runtime_ms BIGINT NOT NULL,
                            billed_seconds BIGINT NOT NULL,
                            price_per_second DOUBLE PRECISION NOT NULL,
                            license_cost DOUBLE PRECISION NOT NULL,
                            total_cost DOUBLE PRECISION NOT NULL,
                            created_at_epoch_ms BIGINT NOT NULL
                        )
                        """.trimIndent()
                    )
                    statement.execute(
                        """
                        CREATE INDEX IF NOT EXISTS idx_${resolvedTableName}_task
                        ON $resolvedTableName (task_id, created_at_epoch_ms)
                        """.trimIndent()
                    )
                    statement.execute(
                        """
                        CREATE INDEX IF NOT EXISTS idx_${resolvedTableName}_scope
                        ON $resolvedTableName (budget_scope, created_at_epoch_ms)
                        """.trimIndent()
                    )
                }
            }
            initialized.set(true)
        }
    }

    /**
     * 成本账本表定义
     *
     * 定义存储成本消费记录的数据库表结构。
     *
     * Cost Ledger Table Definition
     *
     * Defines the database table structure for storing cost consumption records.
     *
     * @param tableName 表名称
     */
    private class CostLedgerTable(tableName: String) : org.ktorm.schema.Table<Nothing>(tableName) {
        val taskId = varchar("task_id")
        val budgetScope = varchar("budget_scope")
        val sliceId = varchar("slice_id")
        val nodeId = varchar("node_id")
        val runtimeMs = long("runtime_ms")
        val billedSeconds = long("billed_seconds")
        val pricePerSecond = double("price_per_second")
        val licenseCost = double("license_cost")
        val totalCost = double("total_cost")
        val createdAtEpochMs = long("created_at_epoch_ms")
    }
}
