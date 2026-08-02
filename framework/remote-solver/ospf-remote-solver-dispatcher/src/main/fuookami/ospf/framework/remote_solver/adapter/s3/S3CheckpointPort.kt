/*
 * S3 检查点端口适配器
 *
 * S3 Checkpoint Port Adapter
 *
 * 该模块提供基于 S3/MinIO 的检查点持久化存储实现，用于保存和检索求解任务的检查点数据。
 * This module provides S3/MinIO-based checkpoint persistence storage implementation
 * for saving and retrieving checkpoint data of solving tasks.
 */
@file:OptIn(kotlin.time.ExperimentalTime::class)

package fuookami.ospf.framework.remote_solver.adapter.s3

import fuookami.ospf.framework.remote_solver.protocol.domain.CheckpointMetadata
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskId
import fuookami.ospf.framework.remote_solver.protocol.port.CheckpointPort
import io.minio.GetObjectArgs
import io.minio.MinioClient
import io.minio.PutObjectArgs
import java.util.Base64
import java.io.ByteArrayInputStream
import kotlin.time.Instant

/**
 * S3 检查点端口
 *
 * S3 Checkpoint Port
 *
 * 该类实现了 [CheckpointPort] 接口，使用 S3/MinIO 对象存储来持久化检查点元数据。
 * 检查点数据以 TSV 格式存储，每个任务对应一个文件，支持版本保留策略。
 *
 * This class implements [CheckpointPort] interface, using S3/MinIO object storage
 * to persist checkpoint metadata. Checkpoint data is stored in TSV format,
 * with one file per task, supporting version retention policy.
 *
 * 存储结构：
 * Storage structure:
 * - 文件路径: {rootPrefix}/{taskId}.checkpoint.log
 * - 每行格式: createdAt\tsliceId\tpath\tversion\tetag (Base64 编码)
 *
 * @param minio MinioClient 客户端实例，用于与 S3/MinIO 服务通信。
 *              MinioClient instance for communicating with S3/MinIO service.
 * @param bucket S3/MinIO 存储桶名称。
 *               S3/MinIO bucket name.
 * @param rootPrefix 存储根路径前缀，默认为 "checkpoints"。
 *                   Storage root path prefix, defaults to "checkpoints".
 * @param maxRetainedPerTask 每个任务最多保留的检查点数量，0 表示不限制。
 *                           Maximum number of checkpoints retained per task, 0 means unlimited.
 */
class S3CheckpointPort(
    private val minio: MinioClient,
    private val bucket: String,
    rootPrefix: String = "checkpoints",
    private val maxRetainedPerTask: Int = 0
) : CheckpointPort {
    private val root = normalizePrefix(rootPrefix)
    private val encoder = Base64.getUrlEncoder().withoutPadding()
    private val decoder = Base64.getUrlDecoder()

    /**
     * 保存检查点元数据
     *
     * Save checkpoint metadata
     *
     * 将检查点元数据追加到对应任务的检查点文件中。如果设置了保留数量限制，
     * 会自动清理最早的检查点记录。
     *
     * Appends checkpoint metadata to the corresponding task's checkpoint file.
     * If retention limit is set, automatically cleans up earliest checkpoint records.
     *
     * @param metadata 检查点元数据，包含任务 ID、切片 ID、对象引用和创建时间。
     *                 Checkpoint metadata containing task ID, slice ID, object reference and creation time.
     */
    override suspend fun save(metadata: CheckpointMetadata) {
        val key = taskKey(metadata.taskId.value)
        val existing = list(metadata.taskId).toMutableList()
        existing += metadata
        val retained = if (maxRetainedPerTask <= 0 || existing.size <= maxRetainedPerTask) {
            existing.sortedBy { it.createdAtEpochMs }
        } else {
            existing.sortedBy { it.createdAtEpochMs }.takeLast(maxRetainedPerTask)
        }
        val body = if (retained.isEmpty()) {
            ""
        } else {
            retained.joinToString(separator = "\n", transform = ::encodeLine) + "\n"
        }
        val bytes = body.toByteArray()
        minio.putObject(
            PutObjectArgs.builder()
                .bucket(bucket)
                .`object`(key)
                .stream(ByteArrayInputStream(bytes), bytes.size.toLong(), -1)
                .build()
        )
    }

    /**
     * 获取最新检查点
     *
     * Get latest checkpoint
     *
     * 从指定任务的检查点列表中返回创建时间最新的检查点元数据。
     * Returns the most recently created checkpoint metadata from the specified task's checkpoint list.
     *
     * @param taskId 任务 ID。
     *               Task ID.
     * @return 最新的检查点元数据，如果不存在则返回 null。
     *         Latest checkpoint metadata, or null if none exists.
     */
    override suspend fun latest(taskId: TaskId): CheckpointMetadata? =
        list(taskId).maxByOrNull { it.createdAtEpochMs }

    /**
     * 列出任务的所有检查点
     *
     * List all checkpoints of a task
     *
     * 从 S3/MinIO 读取指定任务的检查点文件，解析并返回所有检查点元数据，
     * 按创建时间升序排列。
     *
     * Reads the specified task's checkpoint file from S3/MinIO, parses and returns
     * all checkpoint metadata, sorted by creation time ascending.
     *
     * @param taskId 任务 ID。
     *               Task ID.
     * @return 检查点元数据列表，按创建时间升序排列。
     *         List of checkpoint metadata sorted by creation time ascending.
     */
    override suspend fun list(taskId: TaskId): List<CheckpointMetadata> {
        val raw = readStringOrNull(taskKey(taskId.value)) ?: return emptyList()
        return raw
            .lineSequence()
            .filter { it.isNotBlank() }
            .mapNotNull { parseLine(taskId.value, it) }
            .sortedBy { it.createdAtEpochMs }
            .toList()
    }

    /**
     * 构建任务检查点文件的存储键
     *
     * Build storage key for task checkpoint file
     *
     * @param taskId 任务 ID。
     *               Task ID.
     * @return S3 对象键路径。
     *         S3 object key path.
     */
    private fun taskKey(taskId: String): String =
        "$root/${sanitize(taskId)}.checkpoint.log"

    /**
     * 解析单行检查点数据
     *
     * Parse single line checkpoint data
     *
     * 将 TSV 格式的行数据解析为 [CheckpointMetadata] 对象。
     * Parses TSV format line data into [CheckpointMetadata] object.
     *
     * @param taskId 任务 ID。
     *               Task ID.
     * @param line TSV 格式的行字符串。
     *             TSV format line string.
     * @return 解析后的检查点元数据，如果格式无效则返回 null。
     *        Parsed checkpoint metadata, or null if format is invalid.
     */
    private fun parseLine(taskId: String, line: String): CheckpointMetadata? {
        val parts = line.split('\t')
        if (parts.size < 5) {
            return null
        }
        val createdAt = parts[0].toLongOrNull() ?: return null
        val sliceId = decode(parts[1])
        val path = decode(parts[2])
        val version = decodeNullable(parts[3])
        val etag = decodeNullable(parts[4])
        return CheckpointMetadata(
            taskId = TaskId.of(taskId),
            sliceId = fuookami.ospf.framework.remote_solver.protocol.domain.SliceId.of(sliceId),
            ref = ObjectRef.of(path = path, version = version, etag = etag),
            createdAt = Instant.fromEpochMilliseconds(createdAt),
            schemaVersion = parts.getOrNull(5)?.let(::decodeNullable) ?: "1.0",
            modelFingerprint = parts.getOrNull(6)?.let(::decodeNullable),
            configurationFingerprint = parts.getOrNull(7)?.let(::decodeNullable),
            solverFingerprint = parts.getOrNull(8)?.let(::decodeNullable),
            integritySha256 = parts.getOrNull(9)?.let(::decodeNullable)
        )
    }

    /**
     * 将检查点元数据编码为 TSV 行
     *
     * Encode checkpoint metadata to TSV line
     *
     * @param metadata 检查点元数据。
     *                 Checkpoint metadata.
     * @return TSV 格式的行字符串。
     *         TSV format line string.
     */
    private fun encodeLine(metadata: CheckpointMetadata): String =
        buildString {
            append(metadata.createdAtEpochMs)
            append('\t')
            append(encode(metadata.sliceId.value))
            append('\t')
            append(encode(metadata.ref.path.value))
            append('\t')
            append(encodeNullable(metadata.ref.version?.value))
            append('\t')
            append(encodeNullable(metadata.ref.etag?.value))
            append('\t')
            append(encode(metadata.schemaVersion))
            append('\t')
            append(encodeNullable(metadata.modelFingerprint))
            append('\t')
            append(encodeNullable(metadata.configurationFingerprint))
            append('\t')
            append(encodeNullable(metadata.solverFingerprint))
            append('\t')
            append(encodeNullable(metadata.integritySha256))
        }

    /**
     * 读取 S3 对象内容为字符串
     *
     * Read S3 object content as string
     *
     * @param key S3 对象键。
     *            S3 object key.
     * @return 对象内容字符串，如果对象不存在或读取失败则返回 null。
     *         Object content string, or null if object doesn't exist or read fails.
     */
    private fun readStringOrNull(key: String): String? =
        try {
            minio.getObject(
                GetObjectArgs.builder()
                    .bucket(bucket)
                    .`object`(key)
                    .build()
            ).use { it.readBytes().decodeToString() }
        } catch (_: Exception) {
            null
        }

    /**
     * 清理字符串中的非法字符
     *
     * Sanitize illegal characters in string
     *
     * 将非字母数字字符（除 . _ - 外）替换为下划线，确保生成的文件名安全。
     * Replaces non-alphanumeric characters (except . _ -) with underscore to ensure safe filenames.
     *
     * @param value 原始字符串。
     *              Original string.
     * @return 清理后的字符串。
     *         Sanitized string.
     */
    private fun sanitize(value: String): String =
        value.replace(Regex("[^A-Za-z0-9._-]"), "_")

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
        value.trim().trim('/').ifEmpty { "checkpoints" }

    /**
     * Base64 URL 安全编码
     *
     * Base64 URL-safe encoding
     *
     * @param value 原始字符串。
     *              Original string.
     * @return Base64 URL 安全编码后的字符串。
     *         Base64 URL-safe encoded string.
     */
    private fun encode(value: String): String =
        encoder.encodeToString(value.toByteArray())

    /**
     * 编码可能为空的值
     *
     * Encode nullable value
     *
     * 空值编码为 "-"，非空值进行 Base64 编码。
     * Null values are encoded as "-", non-null values are Base64 encoded.
     *
     * @param value 可能为空的字符串。
     *              Nullable string.
     * @return 编码后的字符串。
     *         Encoded string.
     */
    private fun encodeNullable(value: String?): String = if (value == null) "-" else encode(value)

    /**
     * Base64 URL 安全解码
     *
     * Base64 URL-safe decoding
     *
     * @param value Base64 编码的字符串。
     *              Base64 encoded string.
     * @return 解码后的原始字符串。
     *         Decoded original string.
     */
    private fun decode(value: String): String = String(decoder.decode(value))

    /**
     * 解码可能为空的值
     *
     * Decode nullable value
     *
     * "-" 解码为 null，其他值进行 Base64 解码。
     * "-" is decoded as null, other values are Base64 decoded.
     *
     * @param value 编码后的字符串。
     *              Encoded string.
     * @return 解码后的原始字符串或 null。
     *         Decoded original string or null.
     */
    private fun decodeNullable(value: String): String? = if (value == "-") null else decode(value)
}
