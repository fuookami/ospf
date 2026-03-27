@file:OptIn(kotlin.time.ExperimentalTime::class)

/**
 * 内存节点状态端口适配器
 *
 * 提供基于内存的计算节点状态管理实现，用于测试和非持久化场景。
 * 支持节点注册、状态查询、计算单元占用/释放和心跳处理。
 *
 * In-memory node state port adapter.
 *
 * Provides memory-based compute node state management implementation for testing and non-persistent scenarios.
 * Supports node registration, state querying, compute unit occupation/release, and heartbeat handling.
 */
package fuookami.ospf.framework.remote_solver.adapter.inmemory

import fuookami.ospf.framework.remote_solver.domain.NodeState
import fuookami.ospf.framework.remote_solver.port.NodeStatePort
import fuookami.ospf.framework.remote_solver.protocol.domain.NodeId
import java.util.concurrent.ConcurrentHashMap
import kotlin.time.Instant

/**
 * 内存节点状态端口实现
 *
 * 使用内存存储节点状态，支持并发访问和状态更新。
 *
 * In-memory node state port implementation.
 *
 * Uses in-memory storage for node states, supporting concurrent access and state updates.
 */
class InMemoryNodeStatePort : NodeStatePort {
    /**
     * 节点状态存储映射
     *
     * 键为节点ID，值为节点状态。
     *
     * Node state storage map.
     *
     * Keyed by node ID, valued by node state.
     */
    private val nodes = ConcurrentHashMap<String, NodeState>()

    /**
     * 列出节点
     *
     * 返回所有或仅在线的节点列表。
     *
     * Lists nodes.
     *
     * Returns list of all or only online nodes.
     *
     * @param onlineOnly 是否仅返回在线节点
     *                   Whether to return only online nodes
     * @return 节点状态列表
     *         Node state list
     */
    override suspend fun listNodes(onlineOnly: Boolean): List<NodeState> =
        nodes.values.filter { !onlineOnly || it.online }

    /**
     * 获取节点状态
     *
     * 根据节点ID获取节点状态。
     *
     * Gets node state.
     *
     * Retrieves node state by node ID.
     *
     * @param nodeId 节点ID
     *               Node ID
     * @return 节点状态，如果不存在则返回null
     *         Node state, returns null if not found
     */
    override suspend fun getNode(nodeId: NodeId): NodeState? = nodes[nodeId.value]

    /**
     * 更新或插入节点
     *
     * 将节点状态存储到内存中，如果已存在则更新。
     *
     * Upserts node.
     *
     * Stores node state in memory, updates if already exists.
     *
     * @param node 节点状态
     *             Node state
     */
    override suspend fun upsertNode(node: NodeState) {
        nodes[node.nodeId.value] = node
    }

    /**
     * 占用计算单元
     *
     * 从指定节点占用一个计算单元。如果节点离线或无可用单元，操作将失败。
     *
     * Occupies compute unit.
     *
     * Occupies one compute unit from the specified node. If the node is offline or has no available units,
     * the operation will fail.
     *
     * @param nodeId 节点ID
     *               Node ID
     * @return 是否成功占用
     *         Whether occupation was successful
     */
    override suspend fun occupyUnit(nodeId: NodeId): Boolean {
        synchronized(nodes) {
            val node = nodes[nodeId.value] ?: return false
            if (!node.online || node.availableUnits <= 0) {
                return false
            }
            nodes[nodeId.value] = node.copy(availableUnits = node.availableUnits - 1)
            return true
        }
    }

    /**
     * 释放计算单元
     *
     * 将一个计算单元返还给指定节点，不超过节点最大并行单元数。
     *
     * Releases compute unit.
     *
     * Returns one compute unit to the specified node, not exceeding the node's maximum parallel units.
     *
     * @param nodeId 节点ID
     *               Node ID
     * @return 是否成功释放
     *         Whether release was successful
     */
    override suspend fun releaseUnit(nodeId: NodeId): Boolean {
        synchronized(nodes) {
            val node = nodes[nodeId.value] ?: return false
            val upperBound = node.profile.parallelUnits
            val recovered = (node.availableUnits + 1).coerceAtMost(upperBound)
            nodes[nodeId.value] = node.copy(availableUnits = recovered)
            return true
        }
    }

    /**
     * 处理心跳
     *
     * 更新节点的心跳时间和在线状态。如果节点之前离线，将恢复所有计算单元。
     *
     * Handles heartbeat.
     *
     * Updates node's heartbeat time and online status. If the node was previously offline,
     * all compute units will be restored.
     *
     * @param nodeId 节点ID
     *               Node ID
     * @param atEpochMs 心跳时间戳（毫秒）
     *                   Heartbeat timestamp in milliseconds
     */
    override suspend fun heartbeat(nodeId: NodeId, at: Instant) {
        synchronized(nodes) {
            val node = nodes[nodeId.value] ?: return
            val recoveredUnits = if (node.online) {
                node.availableUnits
            } else {
                node.profile.parallelUnits
            }
            nodes[nodeId.value] = node.copy(
                availableUnits = recoveredUnits,
                lastHeartbeat = at,
                online = true
            )
        }
    }
}
