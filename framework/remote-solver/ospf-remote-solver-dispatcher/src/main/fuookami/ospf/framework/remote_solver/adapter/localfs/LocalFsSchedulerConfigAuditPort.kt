/*
 * 本地文件系统调度器配置审计端口适配器
 *
 * Local File System Scheduler Config Audit Port Adapter
 *
 * 该模块提供基于本地文件系统的调度器配置审计记录存储实现，
 * 用于记录调度器热加载和回滚操作的历史，以及配置快照的持久化。
 * This module provides local file system-based scheduler configuration audit
 * record storage implementation, for recording history of scheduler hot reload
 * and rollback operations, and persistence of configuration snapshots.
 */

package fuookami.ospf.framework.remote_solver.adapter.localfs

import fuookami.ospf.framework.remote_solver.application.SchedulerHotReloadAuditRecord
import fuookami.ospf.framework.remote_solver.application.SchedulerRuntimeConfig
import fuookami.ospf.framework.remote_solver.port.SchedulerConfigAuditPort
import java.net.URLDecoder
import java.net.URLEncoder
import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.StandardOpenOption
import java.util.Properties

/**
 * 本地文件系统调度器配置审计端口
 *
 * Local File System Scheduler Config Audit Port
 *
 * 该类实现了 [SchedulerConfigAuditPort] 接口，使用本地文件系统存储调度器配置
 * 的审计记录和配置快照。审计记录以 TSV 格式追加写入日志文件，配置快照
 * 以 Java Properties 格式存储。
 *
 * This class implements [SchedulerConfigAuditPort] interface, using local file system
 * to store scheduler configuration audit records and configuration snapshots.
 * Audit records are appended to log file in TSV format, configuration snapshots
 * are stored in Java Properties format.
 *
 * 存储结构：
 * Storage structure:
 * - 审计日志: {rootDir}/audits.log (TSV 格式，每行一条审计记录)
 * - 配置快照: {rootDir}/snapshots/{version}.properties
 *
 * @param rootDir 存储根目录路径，用于存放审计日志和快照文件。
 *                 Storage root directory path for audit logs and snapshot files.
 */
class LocalFsSchedulerConfigAuditPort(
    rootDir: Path
) : SchedulerConfigAuditPort {
    private val root = rootDir.toAbsolutePath().normalize()
    private val lock = Any()
    private val auditsFile = root.resolve("audits.log")
    private val snapshotDir = root.resolve("snapshots")

    init {
        Files.createDirectories(root)
        Files.createDirectories(snapshotDir)
        if (!Files.exists(auditsFile)) {
            Files.createFile(auditsFile)
        }
    }

    /**
     * 追加审计记录
     *
     * Append audit record
     *
     * 将调度器配置变更的审计记录追加写入审计日志文件。记录格式为 TSV，
     * 包含版本号、前一版本号、操作者、生效时间、回滚来源版本和变更集。
     *
     * Appends scheduler configuration change audit record to audit log file.
     * Record format is TSV, containing version, previous version, operator,
     * effective time, rollback source version and change set.
     *
     * @param record 审计记录，包含配置变更的详细信息。
     *                Audit record containing detailed information of configuration change.
     */
    override suspend fun append(record: SchedulerHotReloadAuditRecord) {
        val line = listOf(
            escape(record.version),
            escape(record.previousVersion),
            escape(record.operator),
            record.effectiveAtEpochMs.toString(),
            escape(record.rollbackFromVersion ?: ""),
            escape(encodeMap(record.changeSet))
        ).joinToString("\t")
        synchronized(lock) {
            Files.writeString(
                auditsFile,
                "$line${System.lineSeparator()}",
                StandardOpenOption.CREATE,
                StandardOpenOption.APPEND
            )
        }
    }

    /**
     * 列出审计记录
     *
     * List audit records
     *
     * 从审计日志文件读取所有审计记录，解析并返回最近的指定数量的记录。
     * 记录按时间顺序返回，最新的记录在列表末尾。
     *
     * Reads all audit records from audit log file, parses and returns
     * specified number of recent records. Records are returned in time order,
     * newest records at the end of list.
     *
     * @param limit 返回的最大记录数量。
     *               Maximum number of records to return.
     * @return 审计记录列表，按时间升序排列。
     *         Audit record list sorted by time ascending.
     */
    override suspend fun list(limit: Int): List<SchedulerHotReloadAuditRecord> {
        val safeLimit = limit.coerceAtLeast(1)
        val parsed = synchronized(lock) {
            Files.readAllLines(auditsFile)
                .asSequence()
                .mapNotNull { parseAuditLine(it) }
                .toList()
        }
        if (parsed.size <= safeLimit) {
            return parsed
        }
        return parsed.takeLast(safeLimit)
    }

    /**
     * 保存配置快照
     *
     * Save configuration snapshot
     *
     * 将指定版本的调度器运行时配置保存为 Properties 文件。文件名由版本号
     * 经过安全化处理后生成。
     *
     * Saves specified version's scheduler runtime configuration as Properties file.
     * Filename is generated from version number after sanitization.
     *
     * @param version 配置版本号。
     *                 Configuration version number.
     * @param config 调度器运行时配置对象。
     *                Scheduler runtime configuration object.
     */
    override suspend fun saveSnapshot(version: String, config: SchedulerRuntimeConfig) {
        val file = snapshotDir.resolve("${sanitize(version)}.properties")
        val props = Properties().apply {
            setProperty("simpleTaskQuantumMs", config.simpleTaskQuantumMs.toString())
            setProperty("complexTaskQuantumMs", config.complexTaskQuantumMs.toString())
            setProperty("complexTaskQuantumMinMs", config.complexTaskQuantumMinMs.toString())
            setProperty("complexTaskQuantumMaxMs", config.complexTaskQuantumMaxMs.toString())
            setProperty("complexSolveEstimateMs", config.complexSolveEstimateMs.toString())
            setProperty("complexCheckpointEstimateMs", config.complexCheckpointEstimateMs.toString())
            setProperty("complexQuantumAlpha", config.complexQuantumAlpha.toString())
            setProperty("complexQuantumBeta", config.complexQuantumBeta.toString())
            setProperty("complexQuantumPricePenalty", config.complexQuantumPricePenalty.toString())
            setProperty("complexUrgencyWeight", config.complexUrgencyWeight.toString())
            setProperty("complexWaitingAgeWeight", config.complexWaitingAgeWeight.toString())
            setProperty("complexProgressNeedWeight", config.complexProgressNeedWeight.toString())
            setProperty("complexCostSensitivityWeight", config.complexCostSensitivityWeight.toString())
        }
        synchronized(lock) {
            Files.newOutputStream(file).use { output ->
                props.store(output, "scheduler config snapshot")
            }
        }
    }

    /**
     * 获取配置快照
     *
     * Get configuration snapshot
     *
     * 从文件系统读取指定版本的调度器运行时配置快照。如果文件不存在或解析失败，
     * 返回 null。
     *
     * Reads specified version's scheduler runtime configuration snapshot from file system.
     * Returns null if file doesn't exist or parsing fails.
     *
     * @param version 配置版本号。
     *                 Configuration version number.
     * @return 调度器运行时配置对象，如果不存在则返回 null。
     *         Scheduler runtime configuration object, or null if not exists.
     */
    override suspend fun getSnapshot(version: String): SchedulerRuntimeConfig? {
        val file = snapshotDir.resolve("${sanitize(version)}.properties")
        if (!Files.exists(file)) {
            return null
        }
        val props = Properties()
        synchronized(lock) {
            Files.newInputStream(file).use { input ->
                props.load(input)
            }
        }
        return SchedulerRuntimeConfig(
            simpleTaskQuantumMs = props.getProperty("simpleTaskQuantumMs")?.toLongOrNull() ?: return null,
            complexTaskQuantumMs = props.getProperty("complexTaskQuantumMs")?.toLongOrNull() ?: return null,
            complexTaskQuantumMinMs = props.getProperty("complexTaskQuantumMinMs")?.toLongOrNull() ?: return null,
            complexTaskQuantumMaxMs = props.getProperty("complexTaskQuantumMaxMs")?.toLongOrNull() ?: return null,
            complexSolveEstimateMs = props.getProperty("complexSolveEstimateMs")?.toLongOrNull() ?: return null,
            complexCheckpointEstimateMs = props.getProperty("complexCheckpointEstimateMs")?.toLongOrNull() ?: return null,
            complexQuantumAlpha = props.getProperty("complexQuantumAlpha")?.toDoubleOrNull() ?: return null,
            complexQuantumBeta = props.getProperty("complexQuantumBeta")?.toDoubleOrNull() ?: return null,
            complexQuantumPricePenalty = props.getProperty("complexQuantumPricePenalty")?.toDoubleOrNull() ?: return null,
            complexUrgencyWeight = props.getProperty("complexUrgencyWeight")?.toDoubleOrNull() ?: return null,
            complexWaitingAgeWeight = props.getProperty("complexWaitingAgeWeight")?.toDoubleOrNull() ?: return null,
            complexProgressNeedWeight = props.getProperty("complexProgressNeedWeight")?.toDoubleOrNull() ?: return null,
            complexCostSensitivityWeight = props.getProperty("complexCostSensitivityWeight")?.toDoubleOrNull() ?: return null
        )
    }

    /**
     * 解析审计日志行
     *
     * Parse audit log line
     *
     * 将 TSV 格式的审计日志行解析为 [SchedulerHotReloadAuditRecord] 对象。
     * Parses TSV format audit log line into [SchedulerHotReloadAuditRecord] object.
     *
     * @param line TSV 格式的行字符串。
     *             TSV format line string.
     * @return 解析后的审计记录，如果格式无效则返回 null。
     *         Parsed audit record, or null if format is invalid.
     */
    private fun parseAuditLine(line: String): SchedulerHotReloadAuditRecord? {
        if (line.isBlank()) {
            return null
        }
        val parts = line.split('\t')
        if (parts.size < 6) {
            return null
        }
        val version = unescape(parts[0])
        val previousVersion = unescape(parts[1])
        val operator = unescape(parts[2])
        val effectiveAt = parts[3].toLongOrNull() ?: return null
        val rollbackFrom = unescape(parts[4]).ifBlank { null }
        val changeSet = decodeMap(unescape(parts[5]))
        return SchedulerHotReloadAuditRecord(
            version = version,
            previousVersion = previousVersion,
            operator = operator,
            effectiveAtEpochMs = effectiveAt,
            changeSet = changeSet,
            rollbackFromVersion = rollbackFrom
        )
    }

    /**
     * 编码变更集为 URL 查询字符串格式
     *
     * Encode change set to URL query string format
     *
     * 将键值对映射编码为 "key1=value1&key2=value2" 格式的字符串，
     * 对键和值进行 URL 编码以处理特殊字符。
     *
     * Encodes key-value pairs mapping to "key1=value1&key2=value2" format string,
     * URL encoding keys and values to handle special characters.
     *
     * @param input 变更集映射。
     *               Change set mapping.
     * @return 编码后的字符串。
     *         Encoded string.
     */
    private fun encodeMap(input: Map<String, String>): String =
        input.entries.joinToString("&") { entry ->
            "${urlEncode(entry.key)}=${urlEncode(entry.value)}"
        }

    /**
     * 解码 URL 查询字符串格式为变更集映射
     *
     * Decode URL query string format to change set mapping
     *
     * 将 "key1=value1&key2=value2" 格式的字符串解码为键值对映射，
     * 对键和值进行 URL 解码还原原始内容。
     *
     * Decodes "key1=value1&key2=value2" format string to key-value pairs mapping,
     * URL decoding keys and values to restore original content.
     *
     * @param encoded 编码后的字符串。
     *                 Encoded string.
     * @return 解码后的变更集映射。
     *         Decoded change set mapping.
     */
    private fun decodeMap(encoded: String): Map<String, String> {
        if (encoded.isBlank()) {
            return emptyMap()
        }
        return encoded.split("&")
            .mapNotNull { part ->
                val idx = part.indexOf('=')
                if (idx <= 0) {
                    return@mapNotNull null
                }
                val key = urlDecode(part.substring(0, idx))
                val value = urlDecode(part.substring(idx + 1))
                key to value
            }
            .toMap()
    }

    /**
     * URL 编码
     *
     * URL encode
     *
     * 使用 UTF-8 编码将字符串转换为 URL 安全格式。
     * Converts string to URL-safe format using UTF-8 encoding.
     *
     * @param raw 原始字符串。
     *             Original string.
     * @return URL 编码后的字符串。
     *         URL encoded string.
     */
    private fun urlEncode(raw: String): String =
        URLEncoder.encode(raw, StandardCharsets.UTF_8)

    /**
     * URL 解码
     *
     * URL decode
     *
     * 使用 UTF-8 编码将 URL 安全格式字符串还原为原始内容。
     * Restores URL-safe format string to original content using UTF-8 encoding.
     *
     * @param raw URL 编码后的字符串。
     *             URL encoded string.
     * @return 解码后的原始字符串。
     *         Decoded original string.
     */
    private fun urlDecode(raw: String): String =
        URLDecoder.decode(raw, StandardCharsets.UTF_8)

    /**
     * 安全化版本号字符串
     *
     * Sanitize version string
     *
     * 将版本号中的非法字符替换为下划线，确保生成的文件名安全有效。
     * 仅保留字母、数字、下划线、连字符和点号。
     *
     * Replaces illegal characters in version number with underscore,
     * ensuring generated filename is safe and valid.
     * Only keeps letters, digits, underscore, hyphen and dot.
     *
     * @param version 原始版本号字符串。
     *                 Original version string.
     * @return 安全化后的版本号字符串。
     *         Sanitized version string.
     */
    private fun sanitize(version: String): String =
        version.map { ch ->
            when {
                ch.isLetterOrDigit() || ch == '_' || ch == '-' || ch == '.' -> ch
                else -> '_'
            }
        }.joinToString("")

    /**
     * 转义特殊字符
     *
     * Escape special characters
     *
     * 将反斜杠、制表符、换行符和回车符转义为可打印字符序列，
     * 用于在 TSV 格式中安全存储包含特殊字符的内容。
     *
     * Escapes backslash, tab, newline and carriage return to printable character sequences,
     * for safely storing content with special characters in TSV format.
     *
     * @param raw 原始字符串。
     *             Original string.
     * @return 转义后的字符串。
     *         Escaped string.
     */
    private fun escape(raw: String): String =
        raw.replace("\\", "\\\\")
            .replace("\t", "\\t")
            .replace("\n", "\\n")
            .replace("\r", "\\r")

    /**
     * 反转义特殊字符
     *
     * Unescape special characters
     *
     * 将转义字符序列还原为原始的特殊字符。
     * Restores escaped character sequences to original special characters.
     *
     * @param raw 转义后的字符串。
     *             Escaped string.
     * @return 原始字符串。
     *         Original string.
     */
    private fun unescape(raw: String): String {
        val sb = StringBuilder(raw.length)
        var i = 0
        while (i < raw.length) {
            val ch = raw[i]
            if (ch == '\\' && i + 1 < raw.length) {
                val next = raw[i + 1]
                when (next) {
                    't' -> sb.append('\t')
                    'n' -> sb.append('\n')
                    'r' -> sb.append('\r')
                    '\\' -> sb.append('\\')
                    else -> {
                        sb.append(ch)
                        sb.append(next)
                    }
                }
                i += 2
                continue
            }
            sb.append(ch)
            i += 1
        }
        return sb.toString()
    }
}