/**
 * 任务族端口模块
 *
 * 本模块提供基于 Ktorm 的任务族性能管理功能，
 * 用于追踪不同类型任务在各计算节点上的性能表现，优化任务分配决策。
 *
 * Task Family Port Module
 *
 * This module provides task family performance management functionality based on Ktorm,
 * used for tracking performance of different task types on compute nodes,
 * optimizing task allocation decisions.
 */
package fuookami.ospf.framework.remote_solver.adapter.ktorm

import fuookami.ospf.framework.remote_solver.domain.TaskFamilyId
import fuookami.ospf.framework.remote_solver.domain.TaskFamilyNodePerformance
import fuookami.ospf.framework.remote_solver.port.TaskFamilyPort
import org.ktorm.database.Database
import java.sql.Connection
import java.sql.PreparedStatement
import java.sql.ResultSet
import java.util.concurrent.atomic.AtomicBoolean

/**
 * Ktorm 任务族端口
 *
 * 基于 Ktorm 实现的任务族性能端口，提供性能数据记录、查询和
 * 最优节点推荐功能。使用指数加权移动平均算法更新性能统计。
 *
 * 表结构：remote_solver_task_family_performance
 * 索引：idx_remote_solver_task_family_performance_family (family_signature)
 *
 * Ktorm Task Family Port
 *
 * Task family performance port implemented with Ktorm, providing performance
 * data recording, querying, and best node recommendation functionality.
 * Uses exponential weighted moving average algorithm to update performance statistics.
 *
 * Table schema: remote_solver_task_family_performance
 * Index: idx_remote_solver_task_family_performance_family (family_signature)
 *
 * @param jdbcUrl JDBC 连接 URL
 * @param username 数据库用户名，可选
 * @param password 数据库密码，可选
 * @param tableName 表名称，默认为 remote_solver_task_family_performance
 */
class KtormTaskFamilyPort(
    private val jdbcUrl: String,
    private val username: String? = null,
    private val password: String? = null,
    tableName: String = "remote_solver_task_family_performance"
) : TaskFamilyPort {

    private val resolvedTableName = normalizeTableName(tableName)
    private val initialized = AtomicBoolean(false)
    private val database = Database.connect(
        url = jdbcUrl,
        user = username,
        password = password
    )

    /**
     * 获取任务族性能数据
     *
     * 获取指定任务族在各节点上的性能统计数据。
     *
     * @param familyId 任务族标识
     * @return 节点 ID 到性能数据的映射
     *
     * Gets performance for task family
     *
     * Retrieves performance statistics for the specified task family on each node.
     *
     * @param familyId Task family identifier
     * @return Map from node ID to performance data
     */
    override suspend fun getPerformance(familyId: TaskFamilyId): Map<String, TaskFamilyNodePerformance> {
        ensureSchema()
        return withConnection { connection ->
            connection.prepareStatement(
                """
                SELECT * FROM $resolvedTableName WHERE family_signature = ?
                """.trimIndent()
            ).use { statement ->
                statement.setString(1, familyId.signature)
                statement.executeQuery().use { resultSet ->
                    val map = mutableMapOf<String, TaskFamilyNodePerformance>()
                    while (resultSet.next()) {
                        val perf = mapPerformance(resultSet)
                        map[perf.nodeId] = perf
                    }
                    map
                }
            }
        }
    }

    /**
     * 记录任务性能
     *
     * 记录任务执行的性能数据，使用指数加权移动平均算法更新统计。
     *
     * @param familyId 任务族标识
     * @param nodeId 执行任务的节点 ID
     * @param runtimeMs 任务运行时间（毫秒）
     * @param cost 任务成本
     * @param success 任务是否成功完成
     *
     * Records performance
     *
     * Records task execution performance data, using exponential weighted moving
     * average algorithm to update statistics.
     *
     * @param familyId Task family identifier
     * @param nodeId Node ID that executed the task
     * @param runtimeMs Task runtime (milliseconds)
     * @param cost Task cost
     * @param success Whether task completed successfully
     */
    override suspend fun recordPerformance(
        familyId: TaskFamilyId,
        nodeId: String,
        runtimeMs: Long,
        cost: Double,
        success: Boolean
    ) {
        ensureSchema()
        withConnection { connection ->
            connection.autoCommit = false
            try {
                val existing = connection.prepareStatement(
                    """
                    SELECT sample_count, avg_runtime_ms, avg_cost_per_task, success_rate
                    FROM $resolvedTableName
                    WHERE family_signature = ? AND node_id = ?
                    """.trimIndent()
                ).use { statement ->
                    statement.setString(1, familyId.signature)
                    statement.setString(2, nodeId)
                    statement.executeQuery().use { resultSet ->
                        if (resultSet.next()) {
                            val oldCount = resultSet.getInt("sample_count")
                            val oldRuntime = resultSet.getDouble("avg_runtime_ms")
                            val oldCost = resultSet.getDouble("avg_cost_per_task")
                            val oldSuccessRate = resultSet.getDouble("success_rate")
                            val newCount = oldCount + 1
                            val alpha = 1.0 / newCount.toDouble()
                            Triple(newCount, alpha, Quadruple(oldRuntime, oldCost, oldSuccessRate, alpha))
                        } else {
                            null
                        }
                    }
                }

                if (existing != null) {
                    val (newCount, alpha, oldValues) = existing
                    val (oldRuntime, oldCost, oldSuccessRate, _) = oldValues
                    connection.prepareStatement(
                        """
                        UPDATE $resolvedTableName
                        SET sample_count = ?,
                            avg_runtime_ms = ?,
                            avg_cost_per_task = ?,
                            success_rate = ?,
                            last_updated_epoch_ms = ?
                        WHERE family_signature = ? AND node_id = ?
                        """.trimIndent()
                    ).use { statement ->
                        statement.setInt(1, newCount)
                        statement.setDouble(2, oldRuntime * (1 - alpha) + runtimeMs.toDouble() * alpha)
                        statement.setDouble(3, oldCost * (1 - alpha) + cost * alpha)
                        statement.setDouble(4, oldSuccessRate * (1 - alpha) + (if (success) 1.0 else 0.0) * alpha)
                        statement.setLong(5, System.currentTimeMillis())
                        statement.setString(6, familyId.signature)
                        statement.setString(7, nodeId)
                        statement.executeUpdate()
                    }
                } else {
                    connection.prepareStatement(
                        """
                        INSERT INTO $resolvedTableName (
                            family_signature, node_id, sample_count, avg_runtime_ms,
                            avg_cost_per_task, success_rate, performance_score, last_updated_epoch_ms
                        ) VALUES (?, ?, 1, ?, ?, ?, 1.0, ?)
                        """.trimIndent()
                    ).use { statement ->
                        statement.setString(1, familyId.signature)
                        statement.setString(2, nodeId)
                        statement.setDouble(3, runtimeMs.toDouble())
                        statement.setDouble(4, cost)
                        statement.setDouble(5, if (success) 1.0 else 0.0)
                        statement.setLong(6, System.currentTimeMillis())
                        statement.executeUpdate()
                    }
                }
                connection.commit()
            } catch (e: Exception) {
                connection.rollback()
                throw e
            } finally {
                connection.autoCommit = true
            }
        }
    }

    /**
     * 获取任务族最优节点
     *
     * 根据性能数据推荐处理指定任务族的最优节点。
     * 按成功率降序、平均成本升序排序。
     *
     * @param familyId 任务族标识
     * @param candidateNodes 候选节点列表
     * @param minSamples 最小样本数阈值
     * @return 最优节点 ID，如果没有符合条件的节点则返回 null
     *
     * Gets best node for task family
     *
     * Recommends the best node to process the specified task family based on
     * performance data. Sorted by success rate descending, average cost ascending.
     *
     * @param familyId Task family identifier
     * @param candidateNodes List of candidate nodes
     * @param minSamples Minimum sample count threshold
     * @return Best node ID, or null if no qualifying nodes
     */
    override suspend fun getBestNodeForFamily(
        familyId: TaskFamilyId,
        candidateNodes: List<String>,
        minSamples: Int
    ): String? {
        ensureSchema()
        if (candidateNodes.isEmpty()) return null

        val placeholders = candidateNodes.joinToString(",") { "?" }
        return withConnection { connection ->
            connection.prepareStatement(
                """
                SELECT node_id, success_rate, avg_cost_per_task
                FROM $resolvedTableName
                WHERE family_signature = ? AND node_id IN ($placeholders) AND sample_count >= ?
                ORDER BY success_rate DESC, avg_cost_per_task ASC
                LIMIT 1
                """.trimIndent()
            ).use { statement ->
                statement.setString(1, familyId.signature)
                candidateNodes.forEachIndexed { index, nodeId ->
                    statement.setString(2 + index, nodeId)
                }
                statement.setInt(2 + candidateNodes.size, minSamples)
                statement.executeQuery().use { resultSet ->
                    if (resultSet.next()) {
                        resultSet.getString("node_id")
                    } else {
                        null
                    }
                }
            }
        }
    }

    /**
     * 映射性能结果集
     *
     * 将数据库结果集映射为任务族节点性能对象。
     *
     * @param resultSet 数据库结果集
     * @return 任务族节点性能对象
     *
     * Maps performance result set
     *
     * Maps database result set to task family node performance object.
     *
     * @param resultSet Database result set
     * @return Task family node performance object
     */
    private fun mapPerformance(resultSet: ResultSet): TaskFamilyNodePerformance {
        return TaskFamilyNodePerformance(
            familyId = TaskFamilyId(resultSet.getString("family_signature")),
            nodeId = resultSet.getString("node_id"),
            sampleCount = resultSet.getInt("sample_count"),
            avgRuntimeMs = resultSet.getDouble("avg_runtime_ms"),
            avgCostPerTask = resultSet.getDouble("avg_cost_per_task"),
            successRate = resultSet.getDouble("success_rate"),
            performanceScore = resultSet.getDouble("performance_score"),
            lastUpdatedEpochMs = resultSet.getLong("last_updated_epoch_ms")
        )
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
        if (initialized.get()) return
        synchronized(initialized) {
            if (initialized.get()) return
            withConnection { connection ->
                connection.createStatement().use { statement ->
                    statement.execute(
                        """
                        CREATE TABLE IF NOT EXISTS $resolvedTableName (
                            id BIGINT GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY,
                            family_signature VARCHAR(256) NOT NULL,
                            node_id VARCHAR(256) NOT NULL,
                            sample_count INT NOT NULL,
                            avg_runtime_ms DOUBLE PRECISION NOT NULL,
                            avg_cost_per_task DOUBLE PRECISION NOT NULL,
                            success_rate DOUBLE PRECISION NOT NULL,
                            performance_score DOUBLE PRECISION NOT NULL,
                            last_updated_epoch_ms BIGINT NOT NULL,
                            UNIQUE(family_signature, node_id)
                        )
                        """.trimIndent()
                    )
                    statement.execute(
                        """
                        CREATE INDEX IF NOT EXISTS idx_${resolvedTableName}_family
                        ON $resolvedTableName (family_signature)
                        """.trimIndent()
                    )
                }
            }
            initialized.set(true)
        }
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
     * 使用连接执行操作
     *
     * 从数据库获取连接并执行指定操作，确保连接正确关闭。
     *
     * @param block 要执行的操作
     * @return 操作结果
     *
     * Executes with connection
     *
     * Gets a connection from the database and executes the specified operation,
     * ensuring proper connection closure.
     *
     * @param block Operation to execute
     * @return Operation result
     */
    private fun <T> withConnection(block: (Connection) -> T): T {
        database.useConnection { connection ->
            return block(connection)
        }
    }
}

/**
 * 四元组数据类
 *
 * 用于存储四个相关值的辅助数据结构。
 *
 * Quadruple Data Class
 *
 * Helper data structure for storing four related values.
 *
 * @param A 第一个元素类型
 * @param B 第二个元素类型
 * @param C 第三个元素类型
 * @param D 第四个元素类型
 * @param first 第一个元素
 * @param second 第二个元素
 * @param third 第三个元素
 * @param fourth 第四个元素
 */
private data class Quadruple<out A, out B, out C, out D>(
    val first: A,
    val second: B,
    val third: C,
    val fourth: D
)