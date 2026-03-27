/*
 * 本地文件系统检查点端口实现
 * Local File System Checkpoint Port Implementation
 *
 * 提供基于本地文件系统的检查点持久化功能。
 * Supports checkpoint persistence based on local file system.
 * 每个任务维护一个检查点日志文件，支持自动清理历史检查点。
 * Each task maintains a checkpoint log file with automatic cleanup of historical checkpoints.
 */
package fuookami.ospf.framework.remote_solver.adapter.localfs

import fuookami.ospf.framework.remote_solver.protocol.domain.CheckpointMetadata
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskId
import fuookami.ospf.framework.remote_solver.protocol.port.CheckpointPort
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.StandardOpenOption
import java.util.Base64

/**
 * 本地文件系统检查点端口
 * Local File System Checkpoint Port
 *
 * 基于 TSV（制表符分隔值）格式的检查点存储实现。
 * TSV-formatted checkpoint storage implementation.
 * 文件命名格式: {taskId}.checkpoint.log
 * File naming format: {taskId}.checkpoint.log
 * 每行格式: createdAt sliceId path version etag（Base64编码）
 * Line format: createdAt sliceId path version etag (Base64 encoded)
 *
 * @param rootPath 检查点文件的根目录路径
 *                 Root directory path for checkpoint files
 * @param maxRetainedPerTask 每个任务保留的最大检查点数量，0表示不限制
 *                           Maximum checkpoints retained per task, 0 means unlimited
 */
class LocalFsCheckpointPort(
    rootPath: Path,
    private val maxRetainedPerTask: Int = 0
) : CheckpointPort {
    private val root = rootPath.toAbsolutePath().normalize()
    private val encoder = Base64.getUrlEncoder().withoutPadding()
    private val decoder = Base64.getUrlDecoder()

    init {
        Files.createDirectories(root)
    }

    /**
     * 保存检查点元数据
     * Save checkpoint metadata
     *
     * 将检查点信息追加到任务对应的日志文件中。
     * Appends checkpoint information to the task's log file.
     * 如果配置了最大保留数量，会自动清理旧的检查点。
     * If max retention is configured, old checkpoints are automatically cleaned up.
     *
     * @param metadata 检查点元数据，包含任务ID、切片ID、对象引用和创建时间
     *                 Checkpoint metadata containing task ID, slice ID, object reference, and creation time
     */
    override suspend fun save(metadata: CheckpointMetadata) {
        val file = taskFile(metadata.taskId.value)
        Files.createDirectories(file.parent)
        val line = encodeLine(metadata)
        synchronized(this) {
            Files.writeString(
                file,
                "$line\n",
                StandardOpenOption.CREATE,
                StandardOpenOption.APPEND
            )
            trimTaskFile(file, metadata.taskId.value)
        }
    }

    /**
     * 获取指定任务的最新检查点
     * Get the latest checkpoint for a specified task
     *
     * 从任务的检查点日志中返回创建时间最新的检查点。
     * Returns the checkpoint with the latest creation time from the task's log.
     *
     * @param taskId 任务ID
     *               Task ID
     * @return 最新的检查点元数据，如果没有检查点则返回null
     *         Latest checkpoint metadata, or null if no checkpoints exist
     */
    override suspend fun latest(taskId: TaskId): CheckpointMetadata? =
        list(taskId).maxByOrNull { it.createdAtEpochMs }

    /**
     * 获取指定任务的检查点列表
     * Get the checkpoint list for a specified task
     *
     * 从任务的检查点日志中读取所有检查点，按创建时间排序。
     * Reads all checkpoints from the task's log, sorted by creation time.
     *
     * @param taskId 任务ID
     *               Task ID
     * @return 检查点元数据列表，按创建时间升序排列
     *         List of checkpoint metadata, sorted by creation time ascending
     */
    override suspend fun list(taskId: TaskId): List<CheckpointMetadata> {
        val file = taskFile(taskId.value)
        if (!Files.exists(file)) {
            return emptyList()
        }
        return Files.readAllLines(file)
            .asSequence()
            .filter { it.isNotBlank() }
            .mapNotNull { parseLine(taskId.value, it) }
            .sortedBy { it.createdAtEpochMs }
            .toList()
    }

    /**
     * 解析日志行内容
     * Parse log line content
     *
     * 将 TSV 格式的日志行解析为 CheckpointMetadata 对象。
     * Parses a TSV-formatted log line into a CheckpointMetadata object.
     *
     * @param taskId 任务ID
     *               Task ID
     * @param line 日志行内容
     *             Log line content
     * @return 解析后的检查点元数据，解析失败返回null
     *         Parsed checkpoint metadata, or null if parsing fails
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
            taskId = taskId,
            sliceId = sliceId,
            ref = ObjectRef.of(path = path, version = version, etag = etag),
            createdAtEpochMs = createdAt
        )
    }

    /**
     * 获取任务对应的检查点日志文件路径
     * Get the checkpoint log file path for a task
     *
     * @param taskId 任务ID
     *               Task ID
     * @return 检查点日志文件的完整路径
     *         Full path to the checkpoint log file
     */
    private fun taskFile(taskId: String): Path = root.resolve("${sanitize(taskId)}.checkpoint.log")

    /**
     * 将检查点元数据编码为日志行
     * Encode checkpoint metadata as a log line
     *
     * 将元数据转换为 TSV 格式的字符串，各字段使用 Base64 编码。
     * Converts metadata to a TSV-formatted string with Base64-encoded fields.
     *
     * @param metadata 检查点元数据
     *                 Checkpoint metadata
     * @return 编码后的日志行字符串
     *         Encoded log line string
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
        }

    /**
     * 清理任务的检查点日志文件
     * Trim the task's checkpoint log file
     *
     * 当检查点数量超过配置的最大保留数时，删除旧的检查点。
     * Removes old checkpoints when count exceeds configured max retention.
     *
     * @param file 日志文件路径
     *             Log file path
     * @param taskId 任务ID
     *               Task ID
     */
    private fun trimTaskFile(file: Path, taskId: String) {
        if (maxRetainedPerTask <= 0 || !Files.exists(file)) {
            return
        }
        val retained = Files.readAllLines(file)
            .asSequence()
            .filter { it.isNotBlank() }
            .mapNotNull { parseLine(taskId, it) }
            .sortedBy { it.createdAtEpochMs }
            .toList()
            .let { checkpoints ->
                if (checkpoints.size <= maxRetainedPerTask) {
                    checkpoints
                } else {
                    checkpoints.takeLast(maxRetainedPerTask)
                }
            }
        val rewritten = if (retained.isEmpty()) {
            ""
        } else {
            retained.joinToString(separator = "\n", transform = ::encodeLine) + "\n"
        }
        Files.writeString(
            file,
            rewritten,
            StandardOpenOption.CREATE,
            StandardOpenOption.TRUNCATE_EXISTING
        )
    }

    /**
     * 清理文件名中的特殊字符
     * Sanitize special characters in filename
     *
     * 将非安全字符替换为下划线，防止路径注入攻击。
     * Replaces unsafe characters with underscores to prevent path injection attacks.
     *
     * @param value 原始字符串
     *              Original string
     * @return 清理后的安全字符串
     *         Sanitized safe string
     */
    private fun sanitize(value: String): String = value.replace(Regex("[^A-Za-z0-9._-]"), "_")

    /**
     * 编码字符串为 Base64 格式
     * Encode string to Base64 format
     *
     * @param value 原始字符串
     *              Original string
     * @return Base64 编码后的字符串
     *         Base64 encoded string
     */
    private fun encode(value: String): String = encoder.encodeToString(value.toByteArray())

    /**
     * 编码可空字符串
     * Encode nullable string
     *
     * null 值使用 "-" 表示。
     * null values are represented as "-".
     *
     * @param value 可空字符串
     *              Nullable string
     * @return 编码后的字符串或 "-"
     *         Encoded string or "-"
     */
    private fun encodeNullable(value: String?): String = if (value == null) "-" else encode(value)

    /**
     * 解码 Base64 字符串
     * Decode Base64 string
     *
     * @param value Base64 编码的字符串
     *              Base64 encoded string
     * @return 解码后的原始字符串
     *         Decoded original string
     */
    private fun decode(value: String): String = String(decoder.decode(value))

    /**
     * 解码可空字符串
     * Decode nullable string
     *
     * "-" 表示 null 值。
     * "-" represents null value.
     *
     * @param value 编码的字符串或 "-"
     *              Encoded string or "-"
     * @return 解码后的字符串或 null
     *         Decoded string or null
     */
    private fun decodeNullable(value: String): String? = if (value == "-") null else decode(value)
}
