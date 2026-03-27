@file:OptIn(kotlin.time.ExperimentalTime::class)

/*
 * 节点状态端口接口
 *
 * Node State Port Interface
 *
 * 该接口定义了计算节点状态管理的核心抽象，支持节点注册、状态更新和资源管理。
 * This interface defines the core abstraction for compute node state management,
 * supporting node registration, status updates, and resource management.
 *
 * 节点状态端口用于追踪计算节点的健康状态、可用资源和工作负载。
 * The node state port is used to track compute node health status,
 * available resources, and workload.
 *
 * 功能特性：
 * Features:
 * - 节点注册和发现 / Node registration and discovery
 * - 心跳检测 / Heartbeat detection
 * - 计算单元占用和释放 / Compute unit occupation and release
 */
package fuookami.ospf.framework.remote_solver.port

import fuookami.ospf.framework.remote_solver.domain.NodeState
import fuookami.ospf.framework.remote_solver.protocol.domain.NodeId
import kotlin.time.Instant

/**
 * 节点状态端口接口
 *
 * Node State Port Interface
 *
 * 提供节点状态管理功能的端口接口。
 * Port interface providing node state management capabilities.
 */
interface NodeStatePort {
    /**
     * 列出节点
     *
     * Lists compute nodes.
     *
     * 获取节点列表，可指定是否只返回在线节点。
     * Retrieves list of nodes, optionally filtering to online nodes only.
     *
     * @param onlineOnly 是否只返回在线节点（默认：true）/ Whether to return online nodes only (default: true)
     * @return 节点状态列表 / List of node states
     */
    suspend fun listNodes(onlineOnly: Boolean = true): List<NodeState>

    /**
     * 获取单个节点
     *
     * Gets a single node by ID.
     *
     * 根据节点 ID 获取节点的当前状态。
     * Retrieves the current state of a node by its ID.
     *
     * @param nodeId 节点唯一标识符 / Unique node identifier
     * @return 节点状态，如不存在返回 null
     *         Node state, or null if not found
     */
    suspend fun getNode(nodeId: String): NodeState? = getNode(NodeId.of(nodeId))

    suspend fun getNode(nodeId: NodeId): NodeState?

    /**
     * 创建或更新节点
     *
     * Creates or updates a node.
     *
     * 创建新节点或更新现有节点的状态信息。
     * Creates a new node or updates an existing node's state information.
     *
     * @param node 节点状态 / Node state
     */
    suspend fun upsertNode(node: NodeState)

    /**
     * 占用计算单元
     *
     * Occupies a compute unit on a node.
     *
     * 尝试在指定节点上占用一个计算单元。如果节点无可用单元，操作将失败。
     * Attempts to occupy a compute unit on the specified node.
     * Operation fails if no units are available on the node.
     *
     * @param nodeId 节点唯一标识符 / Unique node identifier
     * @return 占用成功返回 true，无可用单元返回 false
     *         true if occupation succeeded, false if no units available
     */
    suspend fun occupyUnit(nodeId: String): Boolean = occupyUnit(NodeId.of(nodeId))

    suspend fun occupyUnit(nodeId: NodeId): Boolean

    /**
     * 释放计算单元
     *
     * Releases a compute unit on a node.
     *
     * 释放指定节点上的一个计算单元。
     * Releases a compute unit on the specified node.
     *
     * @param nodeId 节点唯一标识符 / Unique node identifier
     * @return 释放成功返回 true，失败返回 false
     *         true if release succeeded, false otherwise
     */
    suspend fun releaseUnit(nodeId: String): Boolean = releaseUnit(NodeId.of(nodeId))

    suspend fun releaseUnit(nodeId: NodeId): Boolean

    /**
     * 发送心跳
     *
     * Sends a heartbeat for a node.
     *
     * 更新节点的最后活跃时间，用于判断节点是否在线。
     * Updates the node's last active time, used to determine if node is online.
     *
     * @param nodeId 节点唯一标识符 / Unique node identifier
     * @param atEpochMs 心跳时间的毫秒级时间戳 / Heartbeat timestamp in milliseconds
     */
    suspend fun heartbeat(nodeId: String, atEpochMs: Long) =
        heartbeat(nodeId = NodeId.of(nodeId), at = Instant.fromEpochMilliseconds(atEpochMs))

    suspend fun heartbeat(nodeId: NodeId, at: Instant)
}
