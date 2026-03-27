/**
 * 内存对象存储端口适配器
 *
 * 提供基于内存的对象存储实现，用于测试和非持久化场景。
 * 支持对象的版本管理、路径索引和元数据存储。
 *
 * In-memory object storage port adapter.
 *
 * Provides memory-based object storage implementation for testing and non-persistent scenarios.
 * Supports object version management, path indexing, and metadata storage.
 */
package fuookami.ospf.framework.remote_solver.adapter.inmemory

import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectPath
import fuookami.ospf.framework.remote_solver.protocol.port.ClockPort
import fuookami.ospf.framework.remote_solver.protocol.port.ObjectStoragePort
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicLong

/**
 * 内存对象存储端口实现
 *
 * 使用内存存储对象数据，支持版本控制和路径引用解析。
 *
 * In-memory object storage port implementation.
 *
 * Uses in-memory storage for object data, supporting version control and path reference resolution.
 *
 * @param clock 时钟端口，用于获取当前时间戳
 *              Clock port for obtaining current timestamps
 */
class InMemoryObjectStoragePort(
    private val clock: ClockPort
) : ObjectStoragePort {
    /**
     * 对象存储记录
     *
     * 存储对象的完整信息，包括引用、数据、元数据和创建时间。
     *
     * Object storage record.
     *
     * Stores complete object information including reference, data, metadata, and creation time.
     *
     * @param ref 对象引用，包含路径和版本
     *            Object reference containing path and version
     * @param bytes 对象数据字节
     *              Object data bytes
     * @param metadata 对象元数据
     *                 Object metadata
     * @param createdAtEpochMs 创建时间戳（毫秒）
     *                         Creation timestamp in milliseconds
     */
    private data class Record(
        val ref: ObjectRef,
        val bytes: ByteArray,
        val metadata: Map<String, String>,
        val createdAtEpochMs: Long
    )

    /**
     * 版本计数器
     *
     * 用于生成唯一的版本标识符。
     *
     * Version counter.
     *
     * Used for generating unique version identifiers.
     */
    private val versionCounter = AtomicLong(0)

    /**
     * 对象存储映射
     *
     * 键为对象路径和版本的组合，值为对象记录。
     *
     * Object storage map.
     *
     * Keyed by path and version combination, valued by object record.
     */
    private val objects = ConcurrentHashMap<String, Record>()

    /**
     * 路径最新引用映射
     *
     * 键为路径，值为该路径的最新对象引用。
     *
     * Latest reference by path map.
     *
     * Keyed by path, valued by the latest object reference for that path.
     */
    private val latestRefByPath = ConcurrentHashMap<String, ObjectRef>()

    /**
     * 存储对象
     *
     * 将对象数据存储到指定路径，返回包含版本信息的对象引用。
     * 如果路径为空，将自动生成路径。
     *
     * Stores an object.
     *
     * Stores object data to the specified path, returning an object reference with version information.
     * If the path is empty, a path will be automatically generated.
     *
     * @param path 存储路径，可为空
     *             Storage path, can be empty
     * @param bytes 对象数据字节
     *              Object data bytes
     * @param metadata 对象元数据
     *                 Object metadata
     * @return 对象引用，包含路径和版本
     *         Object reference containing path and version
     */
    override suspend fun put(path: ObjectPath, bytes: ByteArray, metadata: Map<String, String>): ObjectRef {
        val pathValue = path.value
        val normalizedPath = if (pathValue.isBlank()) "object/${versionCounter.incrementAndGet()}" else pathValue
        val version = "v${versionCounter.incrementAndGet()}"
        val ref = ObjectRef.of(path = normalizedPath, version = version)
        val record = Record(ref = ref, bytes = bytes.copyOf(), metadata = metadata, createdAtEpochMs = clock.nowEpochMs())
        objects[recordKey(ref)] = record
        latestRefByPath[normalizedPath] = ref
        return ref
    }

    /**
     * 获取对象数据
     *
     * 根据对象引用获取对象数据。如果引用中没有指定版本，将返回该路径的最新版本。
     *
     * Gets object data.
     *
     * Retrieves object data based on object reference. If the reference doesn't specify a version,
     * returns the latest version for that path.
     *
     * @param ref 对象引用
     *            Object reference
     * @return 对象数据字节，如果不存在则返回null
     *         Object data bytes, returns null if not found
     */
    override suspend fun get(ref: ObjectRef): ByteArray? {
        val resolvedRef = resolveRef(ref) ?: return null
        return objects[recordKey(resolvedRef)]?.bytes?.copyOf()
    }

    /**
     * 删除对象
     *
     * 根据对象引用删除对象。如果引用中没有指定版本，将删除该路径的最新版本。
     *
     * Deletes an object.
     *
     * Deletes an object based on object reference. If the reference doesn't specify a version,
     * deletes the latest version for that path.
     *
     * @param ref 对象引用
     *            Object reference
     * @return 是否成功删除
     *         Whether deletion was successful
     */
    override suspend fun delete(ref: ObjectRef): Boolean {
        val resolvedRef = resolveRef(ref) ?: return false
        val removed = objects.remove(recordKey(resolvedRef)) != null
        if (removed && latestRefByPath[resolvedRef.path.value] == resolvedRef) {
            latestRefByPath.remove(resolvedRef.path.value)
        }
        return removed
    }

    /**
     * 检查对象是否存在
     *
     * 根据对象引用检查对象是否存在。如果引用中没有指定版本，将检查该路径的最新版本。
     *
     * Checks if object exists.
     *
     * Checks if an object exists based on object reference. If the reference doesn't specify a version,
     * checks the latest version for that path.
     *
     * @param ref 对象引用
     *            Object reference
     * @return 是否存在
     *         Whether it exists
     */
    override suspend fun exists(ref: ObjectRef): Boolean {
        val resolvedRef = resolveRef(ref) ?: return false
        return objects.containsKey(recordKey(resolvedRef))
    }

    /**
     * 解析对象引用
     *
     * 如果引用中没有版本信息，则返回该路径的最新引用。
     *
     * Resolves object reference.
     *
     * If the reference doesn't have version information, returns the latest reference for that path.
     *
     * @param ref 原始对象引用
     *            Original object reference
     * @return 解析后的对象引用，如果路径不存在则返回null
     *         Resolved object reference, returns null if path doesn't exist
     */
    private fun resolveRef(ref: ObjectRef): ObjectRef? =
        if (ref.version == null) latestRefByPath[ref.path.value] else ref

    /**
     * 生成存储记录键
     *
     * 将对象引用转换为存储键字符串。
     *
     * Generates storage record key.
     *
     * Converts object reference to storage key string.
     *
     * @param ref 对象引用
     *            Object reference
     * @return 存储键字符串
     *         Storage key string
     */
    private fun recordKey(ref: ObjectRef): String = "${ref.path.value}@${ref.version?.value ?: "latest"}"
}
