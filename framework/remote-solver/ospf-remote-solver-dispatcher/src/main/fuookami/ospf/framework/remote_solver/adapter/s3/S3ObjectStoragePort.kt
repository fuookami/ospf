/*
 * S3 对象存储端口适配器
 *
 * S3 Object Storage Port Adapter
 *
 * 该模块提供基于 S3/MinIO 的对象存储实现，用于存储求解任务的输入数据和输出结果。
 * 支持版本控制和元数据存储，提供完整的对象生命周期管理。
 * This module provides S3/MinIO-based object storage implementation for storing
 * input data and output results of solving tasks. Supports version control and
 * metadata storage, providing complete object lifecycle management.
 */

package fuookami.ospf.framework.remote_solver.adapter.s3

import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectPath
import fuookami.ospf.framework.remote_solver.protocol.port.ClockPort
import fuookami.ospf.framework.remote_solver.protocol.port.ObjectStoragePort
import io.minio.GetObjectArgs
import io.minio.MinioClient
import io.minio.PutObjectArgs
import io.minio.RemoveObjectArgs
import io.minio.StatObjectArgs
import java.util.concurrent.atomic.AtomicLong
import java.io.ByteArrayInputStream

/**
 * S3 对象存储端口
 *
 * S3 Object Storage Port
 *
 * 该类实现了 [ObjectStoragePort] 接口，使用 S3/MinIO 提供对象存储服务。
 * 支持版本化存储，每个对象可以存储多个版本，通过 `.latest` 指针文件
 * 标记最新版本。
 *
 * This class implements [ObjectStoragePort] interface, using S3/MinIO to provide
 * object storage service. Supports versioned storage, each object can store
 * multiple versions, marked with `.latest` pointer file for the latest version.
 *
 * 存储结构：
 * Storage structure:
 * - 数据文件: {rootPrefix}/{path}/{version}.data
 * - 元数据文件: {rootPrefix}/{path}/{version}.meta
 * - 最新版本指针: {rootPrefix}/{path}/.latest
 *
 * @param minio MinioClient 客户端实例，用于与 S3/MinIO 服务通信。
 *              MinioClient instance for communicating with S3/MinIO service.
 * @param clock 时钟端口，用于生成时间戳版本的版本标识。
 *              Clock port for generating timestamp-based version identifiers.
 * @param bucket S3/MinIO 存储桶名称。
 *               S3/MinIO bucket name.
 * @param rootPrefix 存储根路径前缀，默认为 "objects"。
 *                   Storage root path prefix, defaults to "objects".
 */
class S3ObjectStoragePort(
    private val minio: MinioClient,
    private val clock: ClockPort,
    private val bucket: String,
    private val rootPrefix: String = "objects"
) : ObjectStoragePort {
    private val sequence = AtomicLong(0)
    private val normalizedRoot = normalizePrefix(rootPrefix)

    /**
     * 存储对象
     *
     * Store object
     *
     * 将对象数据存储到 S3/MinIO，生成版本化的对象引用。同时存储元数据文件
     * 和最新版本指针文件。
     *
     * Stores object data to S3/MinIO, generating versioned object reference.
     * Also stores metadata file and latest version pointer file.
     *
     * @param path 对象存储路径，如果为空则自动生成默认路径。
     *             Object storage path, auto-generates default path if empty.
     * @param bytes 对象数据字节数组。
     *              Object data byte array.
     * @param metadata 对象元数据键值对映射。
     *                  Object metadata key-value pairs mapping.
     * @return 存储后的对象引用，包含路径和版本信息。
     *         Stored object reference containing path and version information.
     */
    override suspend fun put(path: ObjectPath, bytes: ByteArray, metadata: Map<String, String>): ObjectRef {
        val normalizedPath = normalizePath(path.value, defaultPath = "object/${clock.nowEpochMs()}-${sequence.incrementAndGet()}")
        val version = "v${clock.nowEpochMs()}-${sequence.incrementAndGet()}"
        val dataKey = dataKey(normalizedPath, version)
        val metaKey = metadataKey(normalizedPath, version)
        val latestKey = latestPointerKey(normalizedPath)

        minio.putObject(
            PutObjectArgs.builder()
                .bucket(bucket)
                .`object`(dataKey)
                .stream(ByteArrayInputStream(bytes), bytes.size.toLong(), -1)
                .userMetadata(metadata)
                .build(),
        )
        val metadataBody = encodeMetadata(metadata).toByteArray()
        minio.putObject(
            PutObjectArgs.builder()
                .bucket(bucket)
                .`object`(metaKey)
                .stream(ByteArrayInputStream(metadataBody), metadataBody.size.toLong(), -1)
                .build(),
        )
        val latestBody = version.toByteArray()
        minio.putObject(
            PutObjectArgs.builder()
                .bucket(bucket)
                .`object`(latestKey)
                .stream(ByteArrayInputStream(latestBody), latestBody.size.toLong(), -1)
                .build(),
        )

        return ObjectRef.of(path = normalizedPath, version = version)
    }

    /**
     * 获取对象数据
     *
     * Get object data
     *
     * 从 S3/MinIO 读取对象数据。如果对象引用未指定版本，则自动解析最新版本。
     *
     * Reads object data from S3/MinIO. If object reference doesn't specify version,
     * automatically resolves to the latest version.
     *
     * @param ref 对象引用，包含路径和可选的版本信息。
     *             Object reference containing path and optional version information.
     * @return 对象数据字节数组，如果对象不存在则返回 null。
     *         Object data byte array, or null if object doesn't exist.
     */
    override suspend fun get(ref: ObjectRef): ByteArray? {
        val resolved = resolveReference(ref) ?: return null
        val key = dataKey(resolved.path.value, resolved.version?.value ?: return null)
        return readBytesOrNull(key)
    }

    /**
     * 删除对象
     *
     * Delete object
     *
     * 从 S3/MinIO 删除对象的数据文件、元数据文件，如果删除的是最新版本，
     * 则同时删除最新版本指针文件。
     *
     * Deletes object's data file and metadata file from S3/MinIO. If deleting
     * the latest version, also deletes the latest version pointer file.
     *
     * @param ref 对象引用，包含路径和可选的版本信息。
     *             Object reference containing path and optional version information.
     * @return 是否成功删除。
     *         Whether deletion was successful.
     */
    override suspend fun delete(ref: ObjectRef): Boolean {
        val resolved = resolveReference(ref) ?: return false
        val version = resolved.version?.value ?: return false
        val path = resolved.path.value
        val dataKey = dataKey(path, version)
        val metaKey = metadataKey(path, version)
        val latestKey = latestPointerKey(path)

        val exists = existsByKey(dataKey)
        if (!exists) {
            return false
        }
        minio.removeObject(RemoveObjectArgs.builder().bucket(bucket).`object`(dataKey).build())
        minio.removeObject(RemoveObjectArgs.builder().bucket(bucket).`object`(metaKey).build())

        val latest = readStringOrNull(latestKey)?.trim()
        if (latest == version) {
            minio.removeObject(RemoveObjectArgs.builder().bucket(bucket).`object`(latestKey).build())
        }
        return true
    }

    /**
     * 检查对象是否存在
     *
     * Check if object exists
     *
     * @param ref 对象引用，包含路径和可选的版本信息。
     *             Object reference containing path and optional version information.
     * @return 对象是否存在。
     *         Whether object exists.
     */
    override suspend fun exists(ref: ObjectRef): Boolean {
        val resolved = resolveReference(ref) ?: return false
        val version = resolved.version?.value ?: return false
        return existsByKey(dataKey(resolved.path.value, version))
    }

    /**
     * 解析对象引用
     *
     * Resolve object reference
     *
     * 如果对象引用未指定版本，通过读取 `.latest` 指针文件解析最新版本。
     * If object reference doesn't specify version, resolves latest version
     * by reading `.latest` pointer file.
     *
     * @param ref 原始对象引用。
     *             Original object reference.
     * @return 解析后的对象引用，包含明确的版本信息；如果无法解析则返回 null。
     *         Resolved object reference with explicit version information, or null if cannot resolve.
     */
    private fun resolveReference(ref: ObjectRef): ObjectRef? {
        if (!ref.version?.value.isNullOrBlank()) {
            return ref
        }
        val normalizedPath = normalizePath(ref.path.value, defaultPath = "")
        if (normalizedPath.isBlank()) {
            return null
        }
        val latest = readStringOrNull(latestPointerKey(normalizedPath))?.trim()
        if (latest.isNullOrBlank()) {
            return null
        }
        return ObjectRef.of(path = normalizedPath, version = latest, etag = ref.etag?.value)
    }

    /**
     * 检查指定键的对象是否存在
     *
     * Check if object with specified key exists
     *
     * @param key S3 对象键。
     *            S3 object key.
     * @return 对象是否存在。
     *         Whether object exists.
     */
    private fun existsByKey(key: String): Boolean =
        try {
            minio.statObject(
                StatObjectArgs.builder()
                    .bucket(bucket)
                    .`object`(key)
                    .build()
            )
            true
        } catch (_: Exception) {
            false
        }

    /**
     * 读取指定键的对象数据
     *
     * Read object data with specified key
     *
     * @param key S3 对象键。
     *            S3 object key.
     * @return 对象数据字节数组，如果对象不存在则返回 null。
     *         Object data byte array, or null if object doesn't exist.
     */
    private fun readBytesOrNull(key: String): ByteArray? =
        try {
            minio.getObject(
                GetObjectArgs.builder()
                    .bucket(bucket)
                    .`object`(key)
                    .build()
            ).use { it.readBytes() }
        } catch (_: Exception) {
            null
        }

    /**
     * 读取指定键的对象内容为字符串
     *
     * Read object content as string with specified key
     *
     * @param key S3 对象键。
     *            S3 object key.
     * @return 对象内容字符串，如果对象不存在则返回 null。
     *         Object content string, or null if object doesn't exist.
     */
    private fun readStringOrNull(key: String): String? =
        readBytesOrNull(key)?.decodeToString()

    /**
     * 构建数据文件的存储键
     *
     * Build storage key for data file
     *
     * @param path 对象路径。
     *             Object path.
     * @param version 版本标识。
     *                Version identifier.
     * @return 数据文件的 S3 对象键。
     *         S3 object key for data file.
     */
    private fun dataKey(path: String, version: String): String =
        "$normalizedRoot/$path/$version.data"

    /**
     * 构建元数据文件的存储键
     *
     * Build storage key for metadata file
     *
     * @param path 对象路径。
     *             Object path.
     * @param version 版本标识。
     *                Version identifier.
     * @return 元数据文件的 S3 对象键。
     *         S3 object key for metadata file.
     */
    private fun metadataKey(path: String, version: String): String =
        "$normalizedRoot/$path/$version.meta"

    /**
     * 构建最新版本指针文件的存储键
     *
     * Build storage key for latest version pointer file
     *
     * @param path 对象路径。
     *             Object path.
     * @return 最新版本指针文件的 S3 对象键。
     *         S3 object key for latest version pointer file.
     */
    private fun latestPointerKey(path: String): String =
        "$normalizedRoot/$path/.latest"

    /**
     * 编码元数据为字符串
     *
     * Encode metadata to string
     *
     * 将元数据映射编码为键=值格式的多行字符串。
     * Encodes metadata map to key=value format multiline string.
     *
     * @param metadata 元数据映射。
     *                  Metadata map.
     * @return 编码后的字符串。
     *         Encoded string.
     */
    private fun encodeMetadata(metadata: Map<String, String>): String =
        metadata.entries.joinToString(separator = "\n") { "${it.key}=${it.value}" }

    /**
     * 规范化路径前缀
     *
     * Normalize path prefix
     *
     * 移除前后斜杠，如果结果为空则使用默认值。
     * Removes leading and trailing slashes, uses default value if result is empty.
     *
     * @param value 原始前缀值。
     *              Original prefix value.
     * @return 规范化后的前缀。
     *         Normalized prefix.
     */
    private fun normalizePrefix(value: String): String =
        value.trim().trim('/').ifEmpty { "objects" }

    /**
     * 规范化对象路径
     *
     * Normalize object path
     *
     * 移除前后斜杠，检查路径遍历攻击，如果结果为空则使用默认路径。
     * Removes leading and trailing slashes, checks for path traversal attacks,
     * uses default path if result is empty.
     *
     * @param path 原始路径。
     *             Original path.
     * @param defaultPath 默认路径，当原始路径为空时使用。
     *                     Default path used when original path is empty.
     * @return 规范化后的路径。
     *         Normalized path.
     * @throws IllegalArgumentException 如果路径包含路径遍历字符（../）。
     *                                   If path contains path traversal characters (../).
     */
    private fun normalizePath(path: String, defaultPath: String): String {
        val normalized = path.trim().trim('/')
        if (normalized.isEmpty()) {
            return defaultPath
        }
        require(!normalized.startsWith("../")) { "Path traversal is not allowed: $path" }
        require(!normalized.contains("/../")) { "Path traversal is not allowed: $path" }
        return normalized
    }
}
