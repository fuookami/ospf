@file:OptIn(kotlin.time.ExperimentalTime::class)

/**
 * 节点状态端口模块
 *
 * 本模块提供基于 Ktorm 的计算节点状态管理功能，
 * 用于远程求解器调度器的节点注册、状态跟踪和资源管理。
 *
 * Node State Port Module
 *
 * This module provides compute node state management functionality based on Ktorm,
 * used for node registration, state tracking, and resource management
 * in the remote solver dispatcher.
 */
package fuookami.ospf.framework.remote_solver.adapter.ktorm

import fuookami.ospf.framework.remote_solver.domain.NodeCapabilityProfile
import fuookami.ospf.framework.remote_solver.domain.NodeState
import fuookami.ospf.framework.remote_solver.port.NodeStatePort
import fuookami.ospf.framework.remote_solver.protocol.domain.NodeId
import fuookami.ospf.framework.remote_solver.protocol.domain.SolverTypeName
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import org.ktorm.database.Database
import java.sql.Connection
import java.sql.ResultSet
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.time.DurationUnit
import kotlin.time.Instant
import kotlin.time.toDuration

/**
 * Ktorm 节点状态端口
 *
 * 基于 Ktorm 实现的节点状态端口，提供节点查询、注册、
 * 资源占用/释放和心跳维护功能。支持节点的动态注册和离线检测。
 *
 * 表结构：remote_solver_node_state
 *
 * Ktorm Node State Port
 *
 * Node state port implemented with Ktorm, providing node querying,
 * registration, resource occupation/release, and heartbeat maintenance
 * functionality. Supports dynamic node registration and offline detection.
 *
 * Table schema: remote_solver_node_state
 *
 * @param jdbcUrl JDBC 连接 URL
 * @param username 数据库用户名，可选
 * @param password 数据库密码，可选
 * @param tableName 表名称，默认为 remote_solver_node_state
 */
class KtormNodeStatePort(
    private val jdbcUrl: String,
    private val username: String? = null,
    private val password: String? = null,
    tableName: String = "remote_solver_node_state"
) : NodeStatePort {
    private val resolvedTableName = normalizeTableName(tableName)
    private val initialized = AtomicBoolean(false)
    private val database = Database.connect(
        url = jdbcUrl,
        user = username,
        password = password
    )

    /**
     * 查询节点列表
     *
     * 获取所有或仅在线的计算节点状态列表。
     *
     * @param onlineOnly 是否只返回在线节点
     * @return 节点状态列表
     *
     * Lists nodes
     *
     * Retrieves all or only online compute node state list.
     *
     * @param onlineOnly Whether to return only online nodes
     * @return List of node states
     */
    override suspend fun listNodes(onlineOnly: Boolean): List<NodeState> {
        ensureSchema()
        return withConnection { connection ->
            val sql = if (onlineOnly) {
                "SELECT * FROM $resolvedTableName WHERE online = TRUE"
            } else {
                "SELECT * FROM $resolvedTableName"
            }
            connection.prepareStatement(sql).use { statement ->
                statement.executeQuery().use { resultSet ->
                    val nodes = mutableListOf<NodeState>()
                    while (resultSet.next()) {
                        nodes += mapNode(resultSet)
                    }
                    nodes
                }
            }
        }
    }

    /**
     * 获取单个节点
     *
     * 根据节点 ID 获取指定计算节点的状态信息。
     *
     * @param nodeId 节点 ID
     * @return 节点状态，如果不存在则返回 null
     *
     * Gets single node
     *
     * Retrieves the state information for the specified compute node by ID.
     *
     * @param nodeId Node ID
     * @return Node state, or null if not found
     */
    override suspend fun getNode(nodeId: NodeId): NodeState? {
        ensureSchema()
        return withConnection { connection ->
            connection.prepareStatement(
                "SELECT * FROM $resolvedTableName WHERE node_id = ?"
            ).use { statement ->
                statement.setString(1, nodeId.value)
                statement.executeQuery().use { resultSet ->
                    if (!resultSet.next()) {
                        return@withConnection null
                    }
                    mapNode(resultSet)
                }
            }
        }
    }

    /**
     * 注册或更新节点
     *
     * 将计算节点信息插入或更新到数据库。如果节点已存在则更新，
     * 不存在则插入新记录。
     *
     * @param node 节点状态信息
     *
     * Upserts node
     *
     * Inserts or updates compute node information to the database.
     * Updates if node exists, inserts new record if not.
     *
     * @param node Node state information
     */
    override suspend fun upsertNode(node: NodeState) {
        ensureSchema()
        withConnection { connection ->
            connection.autoCommit = false
            try {
                val updated = connection.prepareStatement(
                    """
                    UPDATE $resolvedTableName
                    SET solver_type = ?, performance_score = ?, price_per_second = ?, min_billing_unit_seconds = ?,
                        supports_interrupt = ?, supports_checkpoint = ?, supports_warm_start = ?, parallel_units = ?,
                        license_cost_per_slice = ?, supported_model_types = ?, available_units = ?,
                        last_heartbeat_epoch_ms = ?, online = ?
                    WHERE node_id = ?
                    """.trimIndent()
                ).use { statement ->
                    bindNodeForUpdate(statement, node)
                    statement.executeUpdate()
                }
                if (updated == 0) {
                    connection.prepareStatement(
                        """
                        INSERT INTO $resolvedTableName (
                            node_id, solver_type, performance_score, price_per_second, min_billing_unit_seconds,
                            supports_interrupt, supports_checkpoint, supports_warm_start, parallel_units,
                            license_cost_per_slice, supported_model_types, available_units,
                            last_heartbeat_epoch_ms, online
                        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                        """.trimIndent()
                    ).use { statement ->
                        bindNodeForInsert(statement, node)
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
     * 占用计算单元
     *
     * 尝试占用指定节点的一个计算单元。只有在节点在线且有可用单元时才能成功。
     *
     * @param nodeId 节点 ID
     * @return 是否成功占用
     *
     * Occupies compute unit
     *
     * Attempts to occupy one compute unit on the specified node.
     * Only succeeds when node is online and has available units.
     *
     * @param nodeId Node ID
     * @return Whether occupation succeeded
     */
    override suspend fun occupyUnit(nodeId: NodeId): Boolean {
        ensureSchema()
        return withConnection { connection ->
            connection.prepareStatement(
                """
                UPDATE $resolvedTableName
                SET available_units = available_units - 1
                WHERE node_id = ? AND online = TRUE AND available_units > 0
                """.trimIndent()
            ).use { statement ->
                statement.setString(1, nodeId.value)
                statement.executeUpdate() > 0
            }
        }
    }

    /**
     * 释放计算单元
     *
     * 释放指定节点的一个计算单元。可用单元数量不会超过最大并行单元数。
     *
     * @param nodeId 节点 ID
     * @return 是否成功释放
     *
     * Releases compute unit
     *
     * Releases one compute unit on the specified node.
     * Available units count will not exceed maximum parallel units.
     *
     * @param nodeId Node ID
     * @return Whether release succeeded
     */
    override suspend fun releaseUnit(nodeId: NodeId): Boolean {
        ensureSchema()
        return withConnection { connection ->
            connection.prepareStatement(
                """
                UPDATE $resolvedTableName
                SET available_units = CASE
                    WHEN available_units + 1 > parallel_units THEN parallel_units
                    ELSE available_units + 1
                END
                WHERE node_id = ?
                """.trimIndent()
            ).use { statement ->
                statement.setString(1, nodeId.value)
                statement.executeUpdate() > 0
            }
        }
    }

    /**
     * 更新心跳
     *
     * 更新指定节点的心跳时间戳，标记节点为在线状态。
     * 如果节点之前离线，则恢复其可用计算单元数。
     *
     * @param nodeId 节点 ID
     * @param atEpochMs 心跳时间戳（毫秒）
     *
     * Updates heartbeat
     *
     * Updates the heartbeat timestamp for the specified node,
     * marking the node as online. If node was previously offline,
     * restores its available compute units count.
     *
     * @param nodeId Node ID
     * @param atEpochMs Heartbeat timestamp (milliseconds)
     */
    override suspend fun heartbeat(nodeId: NodeId, at: Instant) {
        ensureSchema()
        withConnection { connection ->
            connection.prepareStatement(
                """
                UPDATE $resolvedTableName
                SET last_heartbeat_epoch_ms = ?,
                    online = TRUE,
                    available_units = CASE
                        WHEN online = TRUE THEN available_units
                        ELSE parallel_units
                    END
                WHERE node_id = ?
                """.trimIndent()
            ).use { statement ->
                statement.setLong(1, at.toEpochMilliseconds())
                statement.setString(2, nodeId.value)
                statement.executeUpdate()
            }
        }
    }

    /**
     * 绑定节点插入参数
     *
     * 将节点状态信息绑定到插入语句的参数中。
     *
     * @param statement 预编译语句
     * @param node 节点状态信息
     *
     * Binds node for insert
     *
     * Binds node state information to insert statement parameters.
     *
     * @param statement Prepared statement
     * @param node Node state information
     */
    private fun bindNodeForInsert(statement: java.sql.PreparedStatement, node: NodeState) {
        val profile = node.profile
        statement.setString(1, node.nodeId.value)
        statement.setString(2, profile.solverType.value)
        statement.setDouble(3, profile.performanceScore.toDouble())
        statement.setDouble(4, profile.pricePerSecond.toDouble())
        statement.setLong(5, profile.minBillingUnitSeconds)
        statement.setBoolean(6, profile.supportsInterrupt)
        statement.setBoolean(7, profile.supportsCheckpoint)
        statement.setBoolean(8, profile.supportsWarmStart)
        statement.setInt(9, profile.parallelUnits)
        statement.setDouble(10, profile.licenseCostPerSlice.toDouble())
        statement.setString(11, encodeModelTypes(profile.supportedModelTypes))
        statement.setInt(12, node.availableUnits)
        statement.setLong(13, node.lastHeartbeatEpochMs)
        statement.setBoolean(14, node.online)
    }

    /**
     * 绑定节点更新参数
     *
     * 将节点状态信息绑定到更新语句的参数中。
     *
     * @param statement 预编译语句
     * @param node 节点状态信息
     *
     * Binds node for update
     *
     * Binds node state information to update statement parameters.
     *
     * @param statement Prepared statement
     * @param node Node state information
     */
    private fun bindNodeForUpdate(statement: java.sql.PreparedStatement, node: NodeState) {
        val profile = node.profile
        statement.setString(1, profile.solverType.value)
        statement.setDouble(2, profile.performanceScore.toDouble())
        statement.setDouble(3, profile.pricePerSecond.toDouble())
        statement.setLong(4, profile.minBillingUnitSeconds)
        statement.setBoolean(5, profile.supportsInterrupt)
        statement.setBoolean(6, profile.supportsCheckpoint)
        statement.setBoolean(7, profile.supportsWarmStart)
        statement.setInt(8, profile.parallelUnits)
        statement.setDouble(9, profile.licenseCostPerSlice.toDouble())
        statement.setString(10, encodeModelTypes(profile.supportedModelTypes))
        statement.setInt(11, node.availableUnits)
        statement.setLong(12, node.lastHeartbeatEpochMs)
        statement.setBoolean(13, node.online)
        statement.setString(14, node.nodeId.value)
    }

    /**
     * 映射节点结果集
     *
     * 将数据库结果集映射为节点状态对象。
     *
     * @param resultSet 数据库结果集
     * @return 节点状态对象
     *
     * Maps node result set
     *
     * Maps database result set to node state object.
     *
     * @param resultSet Database result set
     * @return Node state object
     */
    private fun mapNode(resultSet: ResultSet): NodeState {
        val nodeId = resultSet.getString("node_id")
        val semanticNodeId = NodeId.of(nodeId)
        return NodeState(
            nodeId = semanticNodeId,
            profile = NodeCapabilityProfile(
                nodeId = semanticNodeId,
                solverType = SolverTypeName.of(resultSet.getString("solver_type")),
                performanceScore = Flt64(resultSet.getDouble("performance_score")),
                pricePerSecond = Flt64(resultSet.getDouble("price_per_second")),
                minBillingUnit = resultSet.getLong("min_billing_unit_seconds").toDuration(DurationUnit.SECONDS),
                supportsInterrupt = resultSet.getBoolean("supports_interrupt"),
                supportsCheckpoint = resultSet.getBoolean("supports_checkpoint"),
                supportsWarmStart = resultSet.getBoolean("supports_warm_start"),
                parallelUnits = resultSet.getInt("parallel_units"),
                licenseCostPerSlice = Flt64(resultSet.getDouble("license_cost_per_slice")),
                supportedModelTypes = decodeModelTypes(resultSet.getString("supported_model_types"))
            ),
            availableUnits = resultSet.getInt("available_units"),
            lastHeartbeat = Instant.fromEpochMilliseconds(resultSet.getLong("last_heartbeat_epoch_ms")),
            online = resultSet.getBoolean("online")
        )
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
            withConnection { connection ->
                connection.createStatement().use { statement ->
                    statement.execute(
                        """
                        CREATE TABLE IF NOT EXISTS $resolvedTableName (
                            node_id VARCHAR(256) PRIMARY KEY,
                            solver_type VARCHAR(128) NOT NULL,
                            performance_score DOUBLE PRECISION NOT NULL,
                            price_per_second DOUBLE PRECISION NOT NULL,
                            min_billing_unit_seconds BIGINT NOT NULL,
                            supports_interrupt BOOLEAN NOT NULL,
                            supports_checkpoint BOOLEAN NOT NULL,
                            supports_warm_start BOOLEAN NOT NULL,
                            parallel_units INT NOT NULL,
                            license_cost_per_slice DOUBLE PRECISION NOT NULL,
                            supported_model_types TEXT NOT NULL DEFAULT 'LINEAR,QUADRATIC',
                            available_units INT NOT NULL,
                            last_heartbeat_epoch_ms BIGINT NOT NULL,
                            online BOOLEAN NOT NULL
                        )
                        """.trimIndent()
                    )
                    runCatching {
                        statement.execute("ALTER TABLE $resolvedTableName ADD COLUMN supported_model_types TEXT NOT NULL DEFAULT 'LINEAR,QUADRATIC'")
                    }
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

    private fun encodeModelTypes(types: Set<fuookami.ospf.framework.remote_solver.protocol.domain.NormalizedModelType>): String {
        return types.map { it.name }.sorted().joinToString(",").ifBlank { "UNKNOWN" }
    }

    private fun decodeModelTypes(raw: String?): Set<fuookami.ospf.framework.remote_solver.protocol.domain.NormalizedModelType> {
        val values = raw.orEmpty().split(',')
            .mapNotNull { value -> runCatching { fuookami.ospf.framework.remote_solver.protocol.domain.NormalizedModelType.valueOf(value.trim()) }.getOrNull() }
            .toSet()
        return values.ifEmpty {
            setOf(
                fuookami.ospf.framework.remote_solver.protocol.domain.NormalizedModelType.LINEAR,
                fuookami.ospf.framework.remote_solver.protocol.domain.NormalizedModelType.QUADRATIC
            )
        }
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
