/*
 * UUID ID 生成器端口适配器
 *
 * UUID ID Generator Port Adapter
 *
 * 该模块提供基于 UUID 的唯一标识符生成服务实现，用于生成带有前缀的唯一 ID。
 * This module provides a UUID-based unique identifier generation service implementation
 * for generating prefixed unique IDs.
 */

package fuookami.ospf.framework.remote_solver.adapter.infrastructure

import fuookami.ospf.framework.remote_solver.protocol.port.IdGeneratorPort
import java.util.UUID

/**
 * UUID ID 生成器端口
 *
 * UUID ID Generator Port
 *
 * 该类实现了 [IdGeneratorPort] 接口，使用 Java UUID 生成唯一标识符。
 * 生成的 ID 格式为 "{prefix}-{UUID}"，例如 "task-a1b2c3d4-e5f6-7890"。
 *
 * This class implements [IdGeneratorPort] interface, using Java UUID to generate unique identifiers.
 * The generated ID format is "{prefix}-{UUID}", e.g., "task-a1b2c3d4-e5f6-7890".
 *
 * UUID 提供了极高的唯一性保证，适合分布式系统中生成全局唯一标识符。
 * UUID provides extremely high uniqueness guarantee, suitable for generating globally
 * unique identifiers in distributed systems.
 */
class UUIDIdGeneratorPort : IdGeneratorPort {

    /**
     * 生成新的唯一标识符
     *
     * Generate new unique identifier
     *
     * 使用 UUID 生成一个带有指定前缀的唯一 ID。格式为 "{prefix}-{randomUUID}"。
     * Generates a unique ID with specified prefix using UUID. Format is "{prefix}-{randomUUID}".
     *
     * @param prefix ID 前缀，用于标识 ID 类型（如 "task"、"evt"、"slice" 等）。
     *               ID prefix used to identify ID type (e.g., "task", "evt", "slice", etc.).
     * @return 带有前缀的唯一标识符字符串。
     *         Unique identifier string with prefix.
     */
    override fun newId(prefix: String): String = "$prefix-${UUID.randomUUID()}"
}