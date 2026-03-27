@file:OptIn(kotlin.time.ExperimentalTime::class)

/**
 * 任务状态端口模块
 *
 * 本模块提供基于 Ktorm 的任务状态管理功能，
 * 用于远程求解器调度器的任务持久化、切片管理和状态追踪。
 *
 * Task State Port Module
 *
 * This module provides task state management functionality based on Ktorm,
 * used for task persistence, slice management, and state tracking
 * in the remote solver dispatcher.
 */
package fuookami.ospf.framework.remote_solver.adapter.ktorm

import kotlin.time.DurationUnit
import kotlin.time.Instant
import kotlin.time.toDuration
import fuookami.ospf.framework.remote_solver.domain.SliceState
import fuookami.ospf.framework.remote_solver.domain.TaskState
import fuookami.ospf.framework.remote_solver.protocol.domain.BudgetScopeId
import fuookami.ospf.framework.remote_solver.protocol.domain.DispatchId
import fuookami.ospf.framework.remote_solver.protocol.domain.NodeId
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.RequestId
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceId
import fuookami.ospf.framework.remote_solver.protocol.domain.SliceStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.SolvePayload
import fuookami.ospf.framework.remote_solver.protocol.domain.SolveResult
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskComplexity
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskId
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskMeta
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.TenantId
import fuookami.ospf.framework.remote_solver.protocol.domain.TimeSensitivity
import fuookami.ospf.framework.remote_solver.port.TaskStatePort
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import org.ktorm.database.Database
import java.net.URLDecoder
import java.net.URLEncoder
import java.nio.charset.StandardCharsets
import java.sql.Connection
import java.sql.PreparedStatement
import java.sql.ResultSet
import java.util.concurrent.atomic.AtomicBoolean

/**
 * Ktorm 任务状态端口
 *
 * 基于 Ktorm 实现的任务状态端口，提供任务和切片的 CRUD 操作。
 * 支持任务的创建、查询、更新、条件状态转换，以及切片的追加和查询。
 *
 * 表结构：
 * - remote_solver_task_state: 存储任务状态信息
 * - remote_solver_slice_state: 存储切片状态信息
 *
 * 索引：
 * - idx_remote_solver_task_state_tenant: 租户 ID 索引
 * - idx_remote_solver_task_state_tenant_request: 租户 ID 和请求 ID 唯一索引
 * - idx_remote_solver_slice_state_task_dispatch: 任务 ID 和分发 ID 唯一索引
 * - idx_remote_solver_slice_state_task: 任务 ID 索引
 *
 * Ktorm Task State Port
 *
 * Task state port implemented with Ktorm, providing CRUD operations for tasks and slices.
 * Supports task creation, retrieval, update, conditional state transitions,
 * and slice appending and querying.
 *
 * Table schemas:
 * - remote_solver_task_state: stores task state information
 * - remote_solver_slice_state: stores slice state information
 *
 * Indexes:
 * - idx_remote_solver_task_state_tenant: Tenant ID index
 * - idx_remote_solver_task_state_tenant_request: Tenant ID and request ID unique index
 * - idx_remote_solver_slice_state_task_dispatch: Task ID and dispatch ID unique index
 * - idx_remote_solver_slice_state_task: Task ID index
 *
 * @param jdbcUrl JDBC 连接 URL
 * @param username 数据库用户名，可选
 * @param password 数据库密码，可选
 * @param taskTableName 任务表名称，默认为 remote_solver_task_state
 * @param sliceTableName 切片表名称，默认为 remote_solver_slice_state
 */
class KtormTaskStatePort(
    private val jdbcUrl: String,
    private val username: String? = null,
    private val password: String? = null,
    taskTableName: String = "remote_solver_task_state",
    sliceTableName: String = "remote_solver_slice_state"
) : TaskStatePort {
    private val resolvedTaskTableName = normalizeTableName(taskTableName)
    private val resolvedSliceTableName = normalizeTableName(sliceTableName)
    private val initialized = AtomicBoolean(false)
    private val database = Database.connect(
        url = jdbcUrl,
        user = username,
        password = password
    )

    /**
     * 获取任务
     *
     * 根据 ID 获取指定任务的状态信息。
     *
     * @param taskId 任务 ID
     * @return 任务状态，如果不存在则返回 null
     *
     * Gets task
     *
     * Retrieves the state information for the specified task by ID.
     *
     * @param taskId Task ID
     * @return Task state, or null if not found
     */
    override suspend fun getTask(taskId: TaskId): TaskState? {
        ensureSchema()
        return withConnection { connection ->
            connection.prepareStatement(
                """
                SELECT * FROM $resolvedTaskTableName WHERE task_id = ?
                """.trimIndent()
            ).use { statement ->
                statement.setString(1, taskId.value)
                statement.executeQuery().use { resultSet ->
                    if (!resultSet.next()) {
                        return@withConnection null
                    }
                    mapTask(resultSet)
                }
            }
        }
    }

    /**
     * 根据请求 ID 获取任务
     *
     * 根据请求 ID 获取任务状态信息。
     *
     * @param requestId 请求 ID
     * @return 任务状态，如果不存在则返回 null
     *
     * Gets task by request ID
     *
     * Retrieves task state information by request ID.
     *
     * @param requestId Request ID
     * @return Task state, or null if not found
     */
    override suspend fun getTaskByRequestId(requestId: RequestId): TaskState? {
        ensureSchema()
        return withConnection { connection ->
            connection.prepareStatement(
                """
                SELECT * FROM $resolvedTaskTableName WHERE request_id = ?
                """.trimIndent()
            ).use { statement ->
                statement.setString(1, requestId.value)
                statement.executeQuery().use { resultSet ->
                    if (!resultSet.next()) {
                        return@withConnection null
                    }
                    mapTask(resultSet)
                }
            }
        }
    }

    /**
     * 根据租户和请求 ID 获取任务
     *
     * 根据租户 ID 和请求 ID 获取任务状态信息。
     *
     * @param tenantId 租户 ID
     * @param requestId 请求 ID
     * @return 任务状态，如果不存在则返回 null
     *
     * Gets task by tenant and request ID
     *
     * Retrieves task state information by tenant ID and request ID.
     *
     * @param tenantId Tenant ID
     * @param requestId Request ID
     * @return Task state, or null if not found
     */
    override suspend fun getTaskByRequestId(tenantId: TenantId, requestId: RequestId): TaskState? {
        ensureSchema()
        return withConnection { connection ->
            connection.prepareStatement(
                """
                SELECT * FROM $resolvedTaskTableName WHERE tenant_id = ? AND request_id = ?
                """.trimIndent()
            ).use { statement ->
                statement.setString(1, tenantId.value)
                statement.setString(2, requestId.value)
                statement.executeQuery().use { resultSet ->
                    if (!resultSet.next()) {
                        return@withConnection null
                    }
                    mapTask(resultSet)
                }
            }
        }
    }

    /**
     * 注册或更新任务
     *
     * 将任务信息插入或更新到数据库。如果任务已存在则更新，不存在则插入新记录。
     *
     * @param task 任务状态信息
     *
     * Upserts task
     *
     * Inserts or updates task information to the database.
     * Updates if task exists, inserts new record if not.
     *
     * @param task Task state information
     */
    override suspend fun upsertTask(task: TaskState) {
        ensureSchema()
        withConnection { connection ->
            connection.autoCommit = false
            try {
                val updated = connection.prepareStatement(
                    """
                    UPDATE $resolvedTaskTableName
                    SET request_id = ?, tenant_id = ?, status = ?, complexity = ?, time_sensitivity = ?, priority = ?, deadline_epoch_ms = ?,
                        payload_model_path = ?, payload_model_version = ?, payload_config_path = ?, payload_config_version = ?,
                        payload_snapshot_path = ?, payload_snapshot_version = ?, payload_task_meta_solver_type = ?,
                        payload_task_meta_target_type = ?, payload_task_meta_time_limit_ms = ?,
                        payload_task_meta_solution_limit = ?, payload_task_meta_estimated_variable_count = ?,
                        payload_task_meta_estimated_constraint_count = ?, payload_task_meta_historical_runtime_ms = ?,
                        payload_task_meta_metadata = ?, payload_extension = ?, has_latest_result = ?,
                        latest_result_feasible = ?, latest_result_optimal = ?, latest_result_objective_value = ?,
                        latest_result_gap = ?, latest_result_elapsed_ms = ?, latest_result_checkpoint_path = ?,
                        latest_result_checkpoint_version = ?, latest_result_result_path = ?, latest_result_result_version = ?,
                        latest_result_message = ?, latest_result_extension = ?, latest_snapshot_path = ?,
                        latest_snapshot_version = ?, assigned_node_id = ?, created_at_epoch_ms = ?, updated_at_epoch_ms = ?,
                        budget_scope = ?, budget_limit = ?, consumed_cost = ?
                    WHERE task_id = ?
                    """.trimIndent()
                ).use { statement ->
                    bindTask(statement, task, includeTaskIdAtEnd = true)
                    statement.executeUpdate()
                }
                if (updated == 0) {
                    connection.prepareStatement(
                        """
                        INSERT INTO $resolvedTaskTableName (
                            task_id, request_id, tenant_id, status, complexity, time_sensitivity, priority, deadline_epoch_ms,
                            payload_model_path, payload_model_version, payload_config_path, payload_config_version,
                            payload_snapshot_path, payload_snapshot_version, payload_task_meta_solver_type,
                            payload_task_meta_target_type, payload_task_meta_time_limit_ms,
                            payload_task_meta_solution_limit, payload_task_meta_estimated_variable_count,
                            payload_task_meta_estimated_constraint_count, payload_task_meta_historical_runtime_ms,
                            payload_task_meta_metadata, payload_extension, has_latest_result,
                            latest_result_feasible, latest_result_optimal, latest_result_objective_value,
                            latest_result_gap, latest_result_elapsed_ms, latest_result_checkpoint_path,
                            latest_result_checkpoint_version, latest_result_result_path, latest_result_result_version,
                            latest_result_message, latest_result_extension, latest_snapshot_path, latest_snapshot_version,
                            assigned_node_id, created_at_epoch_ms, updated_at_epoch_ms,
                            budget_scope, budget_limit, consumed_cost
                        ) VALUES (
                            ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?
                        )
                        """.trimIndent()
                    ).use { statement ->
                        bindTask(statement, task, includeTaskIdAtEnd = false)
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
     * 条件状态转换
     *
     * 尝试将任务状态从指定状态集合转换到目标状态，实现乐观锁语义。
     *
     * @param taskId 任务 ID
     * @param from 源状态集合
     * @param to 目标状态
     * @param updatedAtEpochMs 更新时间戳（毫秒）
     * @return 是否成功转换
     *
     * Compare and set status
     *
     * Attempts to transition task status from the specified status set to target status,
     * implementing optimistic lock semantics.
     *
     * @param taskId Task ID
     * @param from Source status set
     * @param to Target status
     * @param updatedAtEpochMs Update timestamp (milliseconds)
     * @return Whether transition succeeded
     */
    override suspend fun compareAndSet(
        taskId: TaskId,
        from: Set<TaskStatus>,
        to: TaskStatus,
        updatedAt: Instant
    ): Boolean {
        if (from.isEmpty()) {
            return false
        }
        ensureSchema()
        return withConnection { connection ->
            val placeholders = from.joinToString(",") { "?" }
            connection.prepareStatement(
                """
                UPDATE $resolvedTaskTableName
                SET status = ?, updated_at_epoch_ms = ?
                WHERE task_id = ? AND status IN ($placeholders)
                """.trimIndent()
            ).use { statement ->
                var index = 1
                statement.setString(index++, to.name)
                statement.setLong(index++, updatedAt.toEpochMilliseconds())
                statement.setString(index++, taskId.value)
                from.forEach { status ->
                    statement.setString(index++, status.name)
                }
                statement.executeUpdate() > 0
            }
        }
    }

    /**
     * 查询任务列表
     *
     * 根据状态集合查询任务列表，按优先级降序、创建时间升序排序。
     *
     * @param statuses 状态集合
     * @param limit 返回数量限制
     * @return 任务状态列表
     *
     * Lists tasks
     *
     * Queries task list by status set, sorted by priority descending,
     * creation time ascending.
     *
     * @param statuses Status set
     * @param limit Return count limit
     * @return List of task states
     */
    override suspend fun listTasks(statuses: Set<TaskStatus>, limit: Int): List<TaskState> {
        if (statuses.isEmpty() || limit <= 0) {
            return emptyList()
        }
        ensureSchema()
        return withConnection { connection ->
            val placeholders = statuses.joinToString(",") { "?" }
            connection.prepareStatement(
                """
                SELECT * FROM $resolvedTaskTableName
                WHERE status IN ($placeholders)
                ORDER BY priority DESC, created_at_epoch_ms ASC
                LIMIT ?
                """.trimIndent()
            ).use { statement ->
                var index = 1
                statuses.forEach { status ->
                    statement.setString(index++, status.name)
                }
                statement.setInt(index, limit)
                statement.executeQuery().use { resultSet ->
                    val list = mutableListOf<TaskState>()
                    while (resultSet.next()) {
                        list += mapTask(resultSet)
                    }
                    list
                }
            }
        }
    }

    /**
     * 添加切片
     *
     * 将切片状态信息追加到数据库。如果切片已存在（根据切片 ID 或分发 ID）则忽略。
     *
     * @param slice 切片状态信息
     *
     * Appends slice
     *
     * Appends slice state information to the database.
     * If slice exists (by slice ID or dispatch ID), it is ignored.
     *
     * @param slice Slice state information
     */
    override suspend fun appendSlice(slice: SliceState) {
        ensureSchema()
        withConnection { connection ->
            connection.autoCommit = false
            try {
                val exists = connection.prepareStatement(
                    """
                    SELECT 1 FROM $resolvedSliceTableName
                    WHERE task_id = ? AND (slice_id = ? OR dispatch_id = ?)
                    """.trimIndent()
                ).use { statement ->
                    statement.setString(1, slice.taskId.value)
                    statement.setString(2, slice.sliceId.value)
                    statement.setString(3, slice.dispatchId.value)
                    statement.executeQuery().use { resultSet -> resultSet.next() }
                }
                if (!exists) {
                    connection.prepareStatement(
                        """
                        INSERT INTO $resolvedSliceTableName (
                            slice_id, task_id, dispatch_id, status, node_id, quantum_ms,
                            checkpoint_path, checkpoint_version, result_path, result_version,
                            started_at_epoch_ms, finished_at_epoch_ms, error_message
                        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                        """.trimIndent()
                    ).use { statement ->
                        bindSlice(statement, slice)
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
     * 更新切片
     *
     * 更新切片状态信息。如果切片不存在则插入新记录。
     *
     * @param slice 切片状态信息
     *
     * Updates slice
     *
     * Updates slice state information. If slice does not exist,
     * inserts new record.
     *
     * @param slice Slice state information
     */
    override suspend fun updateSlice(slice: SliceState) {
        ensureSchema()
        withConnection { connection ->
            connection.autoCommit = false
            try {
                val updated = connection.prepareStatement(
                    """
                    UPDATE $resolvedSliceTableName
                    SET task_id = ?, dispatch_id = ?, status = ?, node_id = ?, quantum_ms = ?,
                        checkpoint_path = ?, checkpoint_version = ?, result_path = ?, result_version = ?,
                        started_at_epoch_ms = ?, finished_at_epoch_ms = ?, error_message = ?
                    WHERE slice_id = ?
                    """.trimIndent()
                ).use { statement ->
                    bindSliceForUpdate(statement, slice)
                    statement.executeUpdate()
                }
                if (updated == 0) {
                    connection.prepareStatement(
                        """
                        INSERT INTO $resolvedSliceTableName (
                            slice_id, task_id, dispatch_id, status, node_id, quantum_ms,
                            checkpoint_path, checkpoint_version, result_path, result_version,
                            started_at_epoch_ms, finished_at_epoch_ms, error_message
                        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                        """.trimIndent()
                    ).use { statement ->
                        bindSlice(statement, slice)
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
     * 获取任务切片列表
     *
     * 获取指定任务的所有切片状态信息，按创建顺序排序。
     *
     * @param taskId 任务 ID
     * @return 切片状态列表
     *
     * Gets slices for task
     *
     * Retrieves all slice state information for the specified task,
     * sorted by creation order.
     *
     * @param taskId Task ID
     * @return List of slice states
     */
    override suspend fun getSlices(taskId: TaskId): List<SliceState> {
        ensureSchema()
        return withConnection { connection ->
            connection.prepareStatement(
                """
                SELECT * FROM $resolvedSliceTableName
                WHERE task_id = ?
                ORDER BY created_seq ASC
                """.trimIndent()
            ).use { statement ->
                statement.setString(1, taskId.value)
                statement.executeQuery().use { resultSet ->
                    val list = mutableListOf<SliceState>()
                    while (resultSet.next()) {
                        list += mapSlice(resultSet)
                    }
                    list
                }
            }
        }
    }

    /**
     * 绑定任务参数
     *
     * 将任务状态信息绑定到预编译语句的参数中。
     *
     * @param statement 预编译语句
     * @param task 任务状态信息
     * @param includeTaskIdAtEnd 是否在参数末尾包含任务 ID
     *
     * Binds task parameters
     *
     * Binds task state information to prepared statement parameters.
     *
     * @param statement Prepared statement
     * @param task Task state information
     * @param includeTaskIdAtEnd Whether to include task ID at end of parameters
     */
    private fun bindTask(statement: PreparedStatement, task: TaskState, includeTaskIdAtEnd: Boolean) {
        val payload = task.payload
        var index = 1
        if (!includeTaskIdAtEnd) {
            statement.setString(index++, task.taskId.value)
        }
        statement.setString(index++, task.requestId.value)
        statement.setString(index++, task.tenantId.value)
        statement.setString(index++, task.status.name)
        statement.setString(index++, task.complexity.name)
        statement.setString(index++, task.timeSensitivity.name)
        statement.setInt(index++, task.priority)
        setNullableLong(statement, index++, task.deadline?.toEpochMilliseconds())
        statement.setString(index++, payload.modelRef?.path?.value)
        statement.setString(index++, payload.modelRef?.version?.value)
        statement.setString(index++, payload.configRef?.path?.value)
        statement.setString(index++, payload.configRef?.version?.value)
        statement.setString(index++, payload.snapshotRef?.path?.value)
        statement.setString(index++, payload.snapshotRef?.version?.value)
        statement.setString(index++, payload.taskMeta.solverType?.value)
        statement.setString(index++, payload.taskMeta.targetType?.value)
        setNullableLong(statement, index++, payload.taskMeta.timeLimitMs)
        setNullableInt(statement, index++, payload.taskMeta.solutionLimit)
        setNullableInt(statement, index++, payload.taskMeta.estimatedVariableCount)
        setNullableInt(statement, index++, payload.taskMeta.estimatedConstraintCount)
        setNullableLong(statement, index++, payload.taskMeta.historicalRuntimeMs)
        statement.setString(index++, encodeMap(payload.taskMeta.metadata))
        statement.setString(index++, encodeMap(payload.extension))
        val hasLatestResult = task.latestResult != null
        statement.setBoolean(index++, hasLatestResult)
        statement.setObject(index++, task.latestResult?.feasible)
        statement.setObject(index++, task.latestResult?.optimal)
        statement.setObject(index++, task.latestResult?.objectiveValue)
        statement.setObject(index++, task.latestResult?.gap)
        setNullableLong(statement, index++, task.latestResult?.elapsedMs)
        statement.setString(index++, task.latestResult?.checkpointRef?.path?.value)
        statement.setString(index++, task.latestResult?.checkpointRef?.version?.value)
        statement.setString(index++, task.latestResult?.resultRef?.path?.value)
        statement.setString(index++, task.latestResult?.resultRef?.version?.value)
        statement.setString(index++, task.latestResult?.message)
        statement.setString(index++, encodeMap(task.latestResult?.extension ?: emptyMap()))
        statement.setString(index++, task.latestSnapshotRef?.path?.value)
        statement.setString(index++, task.latestSnapshotRef?.version?.value)
        statement.setString(index++, task.assignedNodeId?.value)
        statement.setLong(index++, task.createdAt.toEpochMilliseconds())
        statement.setLong(index++, task.updatedAt.toEpochMilliseconds())
        statement.setString(index++, task.budgetScope.value)
        statement.setObject(index++, task.budgetLimit?.toDouble())
        statement.setDouble(index++, task.consumedCost.toDouble())
        if (includeTaskIdAtEnd) {
            statement.setString(index, task.taskId.value)
        }
    }

    /**
     * 绑定切片插入参数
     *
     * 将切片状态信息绑定到插入语句的参数中。
     *
     * @param statement 预编译语句
     * @param slice 切片状态信息
     *
     * Binds slice for insert
     *
     * Binds slice state information to insert statement parameters.
     *
     * @param statement Prepared statement
     * @param slice Slice state information
     */
    private fun bindSlice(statement: PreparedStatement, slice: SliceState) {
        statement.setString(1, slice.sliceId.value)
        statement.setString(2, slice.taskId.value)
        statement.setString(3, slice.dispatchId.value)
        statement.setString(4, slice.status.name)
        statement.setString(5, slice.nodeId?.value)
        statement.setLong(6, slice.quantum.inWholeMilliseconds)
        statement.setString(7, slice.checkpointRef?.path?.value)
        statement.setString(8, slice.checkpointRef?.version?.value)
        statement.setString(9, slice.resultRef?.path?.value)
        statement.setString(10, slice.resultRef?.version?.value)
        setNullableLong(statement, 11, slice.startedAt?.toEpochMilliseconds())
        setNullableLong(statement, 12, slice.finishedAt?.toEpochMilliseconds())
        statement.setString(13, slice.error)
    }

    /**
     * 绑定切片更新参数
     *
     * 将切片状态信息绑定到更新语句的参数中。
     *
     * @param statement 预编译语句
     * @param slice 切片状态信息
     *
     * Binds slice for update
     *
     * Binds slice state information to update statement parameters.
     *
     * @param statement Prepared statement
     * @param slice Slice state information
     */
    private fun bindSliceForUpdate(statement: PreparedStatement, slice: SliceState) {
        statement.setString(1, slice.taskId.value)
        statement.setString(2, slice.dispatchId.value)
        statement.setString(3, slice.status.name)
        statement.setString(4, slice.nodeId?.value)
        statement.setLong(5, slice.quantum.inWholeMilliseconds)
        statement.setString(6, slice.checkpointRef?.path?.value)
        statement.setString(7, slice.checkpointRef?.version?.value)
        statement.setString(8, slice.resultRef?.path?.value)
        statement.setString(9, slice.resultRef?.version?.value)
        setNullableLong(statement, 10, slice.startedAt?.toEpochMilliseconds())
        setNullableLong(statement, 11, slice.finishedAt?.toEpochMilliseconds())
        statement.setString(12, slice.error)
        statement.setString(13, slice.sliceId.value)
    }

    /**
     * 映射任务结果集
     *
     * 将数据库结果集映射为任务状态对象。
     *
     * @param resultSet 数据库结果集
     * @return 任务状态对象
     *
     * Maps task result set
     *
     * Maps database result set to task state object.
     *
     * @param resultSet Database result set
     * @return Task state object
     */
    private fun mapTask(resultSet: ResultSet): TaskState {
        val payload = SolvePayload(
            modelRef = ObjectRef.of(
                path = resultSet.getString("payload_model_path"),
                version = resultSet.getString("payload_model_version")
            ),
            configRef = buildRef(
                path = resultSet.getString("payload_config_path"),
                version = resultSet.getString("payload_config_version")
            ),
            snapshotRef = buildRef(
                path = resultSet.getString("payload_snapshot_path"),
                version = resultSet.getString("payload_snapshot_version")
            ),
            taskMeta = TaskMeta(
                solverType = resultSet.getString("payload_task_meta_solver_type"),
                targetType = resultSet.getString("payload_task_meta_target_type"),
                timeLimitMs = nullableLong(resultSet, "payload_task_meta_time_limit_ms"),
                solutionLimit = nullableInt(resultSet, "payload_task_meta_solution_limit"),
                estimatedVariableCount = nullableInt(resultSet, "payload_task_meta_estimated_variable_count"),
                estimatedConstraintCount = nullableInt(resultSet, "payload_task_meta_estimated_constraint_count"),
                historicalRuntimeMs = nullableLong(resultSet, "payload_task_meta_historical_runtime_ms"),
                metadata = decodeMap(resultSet.getString("payload_task_meta_metadata"))
            ),
            extension = decodeMap(resultSet.getString("payload_extension"))
        )
        val hasLatestResult = resultSet.getBoolean("has_latest_result")
        val latestResult = if (!hasLatestResult) {
            null
        } else {
            SolveResult(
                feasible = resultSet.getBoolean("latest_result_feasible"),
                optimal = resultSet.getBoolean("latest_result_optimal"),
                objectiveValue = nullableDouble(resultSet, "latest_result_objective_value"),
                gap = nullableDouble(resultSet, "latest_result_gap"),
                elapsedMs = nullableLong(resultSet, "latest_result_elapsed_ms") ?: 0L,
                checkpointRef = buildRef(
                    path = resultSet.getString("latest_result_checkpoint_path"),
                    version = resultSet.getString("latest_result_checkpoint_version")
                ),
                resultRef = buildRef(
                    path = resultSet.getString("latest_result_result_path"),
                    version = resultSet.getString("latest_result_result_version")
                ),
                message = resultSet.getString("latest_result_message"),
                extension = decodeMap(resultSet.getString("latest_result_extension"))
            )
        }
        return TaskState(
            taskId = TaskId.of(resultSet.getString("task_id")),
            requestId = RequestId.of(resultSet.getString("request_id")),
            tenantId = TenantId.of(resultSet.getString("tenant_id")),
            status = TaskStatus.valueOf(resultSet.getString("status")),
            complexity = TaskComplexity.valueOf(resultSet.getString("complexity")),
            timeSensitivity = TimeSensitivity.valueOf(resultSet.getString("time_sensitivity")),
            priority = resultSet.getInt("priority"),
            deadline = nullableLong(resultSet, "deadline_epoch_ms")?.let { Instant.fromEpochMilliseconds(it) },
            payload = payload,
            latestResult = latestResult,
            latestSnapshotRef = buildRef(
                path = resultSet.getString("latest_snapshot_path"),
                version = resultSet.getString("latest_snapshot_version")
            ),
            assignedNodeId = resultSet.getString("assigned_node_id")?.let { NodeId.of(it) },
            createdAt = Instant.fromEpochMilliseconds(resultSet.getLong("created_at_epoch_ms")),
            updatedAt = Instant.fromEpochMilliseconds(resultSet.getLong("updated_at_epoch_ms")),
            budgetScope = BudgetScopeId.of(resultSet.getString("budget_scope")),
            budgetLimit = nullableDouble(resultSet, "budget_limit")?.let { Flt64(it) },
            consumedCost = Flt64(resultSet.getDouble("consumed_cost"))
        )
    }

    /**
     * 映射切片结果集
     *
     * 将数据库结果集映射为切片状态对象。
     *
     * @param resultSet 数据库结果集
     * @return 切片状态对象
     *
     * Maps slice result set
     *
     * Maps database result set to slice state object.
     *
     * @param resultSet Database result set
     * @return Slice state object
     */
    private fun mapSlice(resultSet: ResultSet): SliceState =
        SliceState(
            sliceId = SliceId.of(resultSet.getString("slice_id")),
            taskId = TaskId.of(resultSet.getString("task_id")),
            dispatchId = DispatchId.of(resultSet.getString("dispatch_id")),
            status = SliceStatus.valueOf(resultSet.getString("status")),
            nodeId = resultSet.getString("node_id")?.let { NodeId.of(it) },
            quantum = resultSet.getLong("quantum_ms").toDuration(DurationUnit.MILLISECONDS),
            checkpointRef = buildRef(
                path = resultSet.getString("checkpoint_path"),
                version = resultSet.getString("checkpoint_version")
            ),
            resultRef = buildRef(
                path = resultSet.getString("result_path"),
                version = resultSet.getString("result_version")
            ),
            startedAt = nullableLong(resultSet, "started_at_epoch_ms")?.let { Instant.fromEpochMilliseconds(it) },
            finishedAt = nullableLong(resultSet, "finished_at_epoch_ms")?.let { Instant.fromEpochMilliseconds(it) },
            error = resultSet.getString("error_message")
        )

    /**
     * 构建对象引用
     *
     * 根据路径和版本构建对象引用，如果路径为空则返回 null。
     *
     * @param path 对象路径
     * @param version 对象版本
     * @return 对象引用，如果路径为空则返回 null
     *
     * Builds object reference
     *
     * Builds object reference from path and version.
     * Returns null if path is empty.
     *
     * @param path Object path
     * @param version Object version
     * @return Object reference, or null if path is empty
     */
    private fun buildRef(path: String?, version: String?): ObjectRef? {
        if (path.isNullOrBlank()) {
            return null
        }
        return ObjectRef.of(path = path, version = version)
    }

    /**
     * 获取可空长整型值
     *
     * 从结果集获取可能为 null 的长整型值。
     *
     * @param resultSet 数据库结果集
     * @param column 列名
     * @return 长整型值，如果为 null 则返回 null
     *
     * Gets nullable long value
     *
     * Retrieves potentially null long value from result set.
     *
     * @param resultSet Database result set
     * @param column Column name
     * @return Long value, or null if null
     */
    private fun nullableLong(resultSet: ResultSet, column: String): Long? {
        val value = resultSet.getLong(column)
        return if (resultSet.wasNull()) null else value
    }

    /**
     * 获取可空整型值
     *
     * 从结果集获取可能为 null 的整型值。
     *
     * @param resultSet 数据库结果集
     * @param column 列名
     * @return 整型值，如果为 null 则返回 null
     *
     * Gets nullable int value
     *
     * Retrieves potentially null int value from result set.
     *
     * @param resultSet Database result set
     * @param column Column name
     * @return Int value, or null if null
     */
    private fun nullableInt(resultSet: ResultSet, column: String): Int? {
        val value = resultSet.getInt(column)
        return if (resultSet.wasNull()) null else value
    }

    /**
     * 获取可空双精度浮点值
     *
     * 从结果集获取可能为 null 的双精度浮点值。
     *
     * @param resultSet 数据库结果集
     * @param column 列名
     * @return 双精度浮点值，如果为 null 则返回 null
     *
     * Gets nullable double value
     *
     * Retrieves potentially null double value from result set.
     *
     * @param resultSet Database result set
     * @param column Column name
     * @return Double value, or null if null
     */
    private fun nullableDouble(resultSet: ResultSet, column: String): Double? {
        val value = resultSet.getDouble(column)
        return if (resultSet.wasNull()) null else value
    }

    /**
     * 设置可空长整型参数
     *
     * 将可能为 null 的长整型值设置到预编译语句参数中。
     *
     * @param statement 预编译语句
     * @param index 参数索引
     * @param value 长整型值
     *
     * Sets nullable long parameter
     *
     * Sets potentially null long value to prepared statement parameter.
     *
     * @param statement Prepared statement
     * @param index Parameter index
     * @param value Long value
     */
    private fun setNullableLong(statement: PreparedStatement, index: Int, value: Long?) {
        if (value == null) {
            statement.setObject(index, null)
        } else {
            statement.setLong(index, value)
        }
    }

    /**
     * 设置可空整型参数
     *
     * 将可能为 null 的整型值设置到预编译语句参数中。
     *
     * @param statement 预编译语句
     * @param index 参数索引
     * @param value 整型值
     *
     * Sets nullable int parameter
     *
     * Sets potentially null int value to prepared statement parameter.
     *
     * @param statement Prepared statement
     * @param index Parameter index
     * @param value Int value
     */
    private fun setNullableInt(statement: PreparedStatement, index: Int, value: Int?) {
        if (value == null) {
            statement.setObject(index, null)
        } else {
            statement.setInt(index, value)
        }
    }

    /**
     * 编码映射为字符串
     *
     * 将键值对映射编码为 URL 编码的字符串格式。
     *
     * @param value 要编码的映射
     * @return 编码后的字符串
     *
     * Encodes map to string
     *
     * Encodes a key-value map to a URL-encoded string format.
     *
     * @param value Map to encode
     * @return Encoded string
     */
    private fun encodeMap(value: Map<String, String>): String =
        value.entries
            .sortedBy { it.key }
            .joinToString("&") {
                "${encodeToken(it.key)}=${encodeToken(it.value)}"
            }

    /**
     * 解码字符串为映射
     *
     * 将 URL 编码的字符串解码为键值对映射。
     *
     * @param value 编码的字符串
     * @return 解码后的映射
     *
     * Decodes string to map
     *
     * Decodes a URL-encoded string to a key-value map.
     *
     * @param value Encoded string
     * @return Decoded map
     */
    private fun decodeMap(value: String?): Map<String, String> {
        if (value.isNullOrBlank()) {
            return emptyMap()
        }
        return value
            .split("&")
            .filter { it.isNotBlank() }
            .associate { token ->
                val index = token.indexOf('=')
                if (index < 0) {
                    decodeToken(token) to ""
                } else {
                    decodeToken(token.substring(0, index)) to decodeToken(token.substring(index + 1))
                }
            }
    }

    /**
     * 编码令牌
     *
     * 对字符串进行 URL 编码。
     *
     * @param value 原始字符串
     * @return 编码后的字符串
     *
     * Encodes token
     *
     * Performs URL encoding on a string.
     *
     * @param value Raw string
     * @return Encoded string
     */
    private fun encodeToken(value: String): String =
        URLEncoder.encode(value, StandardCharsets.UTF_8)

    /**
     * 解码令牌
     *
     * 对 URL 编码的字符串进行解码。
     *
     * @param value 编码的字符串
     * @return 解码后的字符串
     *
     * Decodes token
     *
     * Decodes a URL-encoded string.
     *
     * @param value Encoded string
     * @return Decoded string
     */
    private fun decodeToken(value: String): String =
        URLDecoder.decode(value, StandardCharsets.UTF_8)

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
            withConnection { connection ->
                connection.createStatement().use { statement ->
                    statement.execute(
                        """
                        CREATE TABLE IF NOT EXISTS $resolvedTaskTableName (
                            task_id VARCHAR(256) PRIMARY KEY,
                            request_id VARCHAR(256) NOT NULL,
                            tenant_id VARCHAR(256) NOT NULL DEFAULT 'default',
                            status VARCHAR(64) NOT NULL,
                            complexity VARCHAR(64) NOT NULL,
                            time_sensitivity VARCHAR(64) NOT NULL,
                            priority INT NOT NULL,
                            deadline_epoch_ms BIGINT NULL,
                            payload_model_path TEXT NOT NULL,
                            payload_model_version TEXT NULL,
                            payload_config_path TEXT NULL,
                            payload_config_version TEXT NULL,
                            payload_snapshot_path TEXT NULL,
                            payload_snapshot_version TEXT NULL,
                            payload_task_meta_solver_type TEXT NULL,
                            payload_task_meta_target_type TEXT NULL,
                            payload_task_meta_time_limit_ms BIGINT NULL,
                            payload_task_meta_solution_limit INT NULL,
                            payload_task_meta_estimated_variable_count INT NULL,
                            payload_task_meta_estimated_constraint_count INT NULL,
                            payload_task_meta_historical_runtime_ms BIGINT NULL,
                            payload_task_meta_metadata TEXT NOT NULL,
                            payload_extension TEXT NOT NULL,
                            has_latest_result BOOLEAN NOT NULL,
                            latest_result_feasible BOOLEAN NULL,
                            latest_result_optimal BOOLEAN NULL,
                            latest_result_objective_value DOUBLE PRECISION NULL,
                            latest_result_gap DOUBLE PRECISION NULL,
                            latest_result_elapsed_ms BIGINT NULL,
                            latest_result_checkpoint_path TEXT NULL,
                            latest_result_checkpoint_version TEXT NULL,
                            latest_result_result_path TEXT NULL,
                            latest_result_result_version TEXT NULL,
                            latest_result_message TEXT NULL,
                            latest_result_extension TEXT NOT NULL,
                            latest_snapshot_path TEXT NULL,
                            latest_snapshot_version TEXT NULL,
                            assigned_node_id VARCHAR(256) NULL,
                            created_at_epoch_ms BIGINT NOT NULL,
                            updated_at_epoch_ms BIGINT NOT NULL,
                            budget_scope VARCHAR(256) NOT NULL,
                            budget_limit DOUBLE PRECISION NULL,
                            consumed_cost DOUBLE PRECISION NOT NULL
                        )
                        """.trimIndent()
                    )
                    statement.execute(
                        """
                        CREATE INDEX IF NOT EXISTS idx_${resolvedTaskTableName}_tenant
                        ON $resolvedTaskTableName (tenant_id)
                        """.trimIndent()
                    )
                    statement.execute(
                        """
                        CREATE UNIQUE INDEX IF NOT EXISTS idx_${resolvedTaskTableName}_tenant_request
                        ON $resolvedTaskTableName (tenant_id, request_id)
                        """.trimIndent()
                    )
                    statement.execute(
                        """
                        CREATE TABLE IF NOT EXISTS $resolvedSliceTableName (
                            created_seq BIGINT GENERATED BY DEFAULT AS IDENTITY,
                            slice_id VARCHAR(256) PRIMARY KEY,
                            task_id VARCHAR(256) NOT NULL,
                            dispatch_id VARCHAR(256) NOT NULL,
                            status VARCHAR(64) NOT NULL,
                            node_id VARCHAR(256) NULL,
                            quantum_ms BIGINT NOT NULL,
                            checkpoint_path TEXT NULL,
                            checkpoint_version TEXT NULL,
                            result_path TEXT NULL,
                            result_version TEXT NULL,
                            started_at_epoch_ms BIGINT NULL,
                            finished_at_epoch_ms BIGINT NULL,
                            error_message TEXT NULL
                        )
                        """.trimIndent()
                    )
                    statement.execute(
                        """
                        CREATE UNIQUE INDEX IF NOT EXISTS idx_${resolvedSliceTableName}_task_dispatch
                        ON $resolvedSliceTableName (task_id, dispatch_id)
                        """.trimIndent()
                    )
                    statement.execute(
                        """
                        CREATE INDEX IF NOT EXISTS idx_${resolvedSliceTableName}_task
                        ON $resolvedSliceTableName (task_id)
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
