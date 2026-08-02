/** Object-storage backed portable checkpoint store. / 基于对象存储的可移植 checkpoint 存储。 */
package fuookami.ospf.framework.remote_solver.adapter.portable

import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.PortableCheckpointCodec
import fuookami.ospf.framework.remote_solver.protocol.domain.PortableCheckpointEnvelope
import fuookami.ospf.framework.remote_solver.protocol.port.ObjectStoragePort

/**
 * Stores and verifies checkpoint v2 envelopes under a tenant-scoped path.
 * 在租户作用域路径下存储并验证 checkpoint v2 envelope。
 *
 * @property objectStoragePort object storage port / 对象存储端口
 */
class PortableCheckpointStore(
    private val objectStoragePort: ObjectStoragePort
) {
    /**
     * Writes a checkpoint envelope.
     * 写入 checkpoint envelope。
     *
     * @param tenantId 租户 ID / Tenant identifier
     * @param taskId 任务 ID / Task identifier
     * @param envelope checkpoint envelope / checkpoint envelope
     * @return stored reference / 存储引用
     */
    suspend fun put(
        tenantId: String,
        taskId: String,
        envelope: PortableCheckpointEnvelope
    ): ObjectRef {
        val safeTenant = tenantId.trim().ifEmpty { "default" }
        val safeTask = taskId.trim().ifEmpty { "unknown" }
        val bytes = PortableCheckpointCodec.encode(envelope).encodeToByteArray()
        return objectStoragePort.put(
            path = "$safeTenant/checkpoint/$safeTask/${envelope.checkpointId}",
            bytes = bytes,
            metadata = mapOf(
                "contentType" to "application/json",
                "schemaVersion" to envelope.schemaVersion,
                "tenantId" to safeTenant,
                "taskId" to safeTask,
                "checkpointId" to envelope.checkpointId
            )
        )
    }

    /**
     * Reads and verifies a checkpoint envelope.
     * 读取并验证 checkpoint envelope。
     *
     * @param ref object reference / 对象引用
     * @return verified envelope, or null when missing/corrupt / 已验证 envelope；缺失或损坏时返回 null
     */
    suspend fun get(ref: ObjectRef): PortableCheckpointEnvelope? {
        val bytes = objectStoragePort.get(ref) ?: return null
        return PortableCheckpointCodec.decodeCompatibleOrNull(bytes.decodeToString())
    }

    /**
     * Reads a checkpoint only when its path belongs to the tenant.
     * 仅当 checkpoint 路径属于指定租户时读取。
     *
     * @param tenantId 租户 ID / Tenant identifier
     * @param ref object reference / 对象引用
     * @return verified envelope, or null when missing, corrupt, or cross-tenant / 已验证 envelope；缺失、损坏或跨租户时返回 null
     */
    suspend fun get(tenantId: String, ref: ObjectRef): PortableCheckpointEnvelope? {
        val normalizedTenant = tenantId.trim().ifEmpty { "default" }
        if (!ref.path.value.startsWith("$normalizedTenant/checkpoint/")) {
            return null
        }
        return get(ref)
    }
}
