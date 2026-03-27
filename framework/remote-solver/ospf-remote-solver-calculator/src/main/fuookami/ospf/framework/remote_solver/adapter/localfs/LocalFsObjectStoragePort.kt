/*
 * 本地文件系统对象存储端口实现
 * Local File System Object Storage Port Implementation
 *
 * 提供基于本地文件系统的对象存储功能。
 * Supports object storage based on local file system.
 * 支持版本管理、元数据存储和自动清理功能。
 * Supports version management, metadata storage, and automatic cleanup.
 */
package fuookami.ospf.framework.remote_solver.adapter.localfs

import fuookami.ospf.framework.remote_solver.protocol.port.ClockPort
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectPath
import fuookami.ospf.framework.remote_solver.protocol.port.ObjectStoragePort
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import java.nio.file.StandardOpenOption
import java.util.concurrent.atomic.AtomicLong

/**
 * 本地文件系统对象存储端口
 * Local File System Object Storage Port
 *
 * 将对象数据存储在本地文件系统中，支持版本控制。
 * Stores object data in local file system with version control.
 * 存储结构:
 * Storage structure:
 * - {root}/{path}/{version}.data  对象数据文件
 *                                Object data file
 * - {root}/{path}/{version}.meta  对象元数据文件
 *                                Object metadata file
 * - {root}/{path}/.latest         最新版本标记文件
 *                                Latest version marker file
 *
 * @param rootPath 对象存储的根目录路径
 *                 Root directory path for object storage
 * @param clock 时钟端口，用于生成时间戳版本号
 *              Clock port for generating timestamp versions
 */
class LocalFsObjectStoragePort(
    rootPath: Path,
    private val clock: ClockPort
) : ObjectStoragePort {
    private val root = rootPath.toAbsolutePath().normalize()
    private val sequence = AtomicLong(0)

    init {
        Files.createDirectories(root)
    }

    /**
     * 存储对象数据
     * Store object data
     *
     * 将数据写入文件系统，生成带版本号的文件路径。
     * Writes data to file system with versioned file paths.
     * 如果路径为空，自动生成唯一路径。
     * If path is empty, a unique path is auto-generated.
     *
     * @param path 对象的存储路径（相对路径）
     *             Object storage path (relative path)
     * @param bytes 对象数据的字节数组
     *              Object data as byte array
     * @param metadata 对象的元数据键值对
     *                 Object metadata key-value pairs
     * @return 对象引用，包含路径和版本号
     *         Object reference containing path and version
     */
    override suspend fun put(path: ObjectPath, bytes: ByteArray, metadata: Map<String, String>): ObjectRef {
        val pathValue = path.value
        val normalizedPath = if (pathValue.isBlank()) {
            "object/${clock.nowEpochMs()}-${sequence.incrementAndGet()}"
        } else {
            pathValue
        }
        val objectDir = resolveObjectDir(normalizedPath)
        Files.createDirectories(objectDir)
        val version = "v${clock.nowEpochMs()}-${sequence.incrementAndGet()}"
        val dataFile = objectDir.resolve("$version.data")
        val metaFile = objectDir.resolve("$version.meta")
        Files.write(dataFile, bytes, StandardOpenOption.CREATE_NEW)
        val metadataBody = metadata.entries.joinToString(separator = "\n") { "${it.key}=${it.value}" }
        Files.writeString(metaFile, metadataBody, StandardOpenOption.CREATE_NEW)
        Files.writeString(objectDir.resolve(".latest"), version, StandardOpenOption.CREATE, StandardOpenOption.TRUNCATE_EXISTING)
        return ObjectRef.of(path = normalizedPath, version = version)
    }

    /**
     * 获取对象数据
     * Get object data
     *
     * 从文件系统读取指定版本的对象数据。
     * Reads object data of specified version from file system.
     * 如果引用未指定版本，使用最新版本。
     * If reference doesn't specify version, uses latest version.
     *
     * @param ref 对象引用，包含路径和可选的版本号
     *            Object reference containing path and optional version
     * @return 对象数据的字节数组，如果不存在则返回null
     *         Object data as byte array, or null if not found
     */
    override suspend fun get(ref: ObjectRef): ByteArray? {
        val resolved = resolveReference(ref) ?: return null
        val dataFile = dataFileOf(resolved)
        return if (Files.exists(dataFile)) Files.readAllBytes(dataFile) else null
    }

    /**
     * 删除对象
     * Delete object
     *
     * 从文件系统删除指定版本的对象数据文件和元数据文件。
     * Deletes object data file and metadata file of specified version from file system.
     * 如果删除的是最新版本，同时删除最新版本标记文件。
     * If deleting latest version, also deletes the latest version marker file.
     *
     * @param ref 对象引用
     *            Object reference
     * @return 是否成功删除了数据文件
     *         Whether data file was successfully deleted
     */
    override suspend fun delete(ref: ObjectRef): Boolean {
        val resolved = resolveReference(ref) ?: return false
        val objectDir = resolveObjectDir(resolved.path.value)
        val dataFile = dataFileOf(resolved)
        val metaFile = metadataFileOf(resolved)
        var deleted = false
        if (Files.exists(dataFile)) {
            Files.delete(dataFile)
            deleted = true
        }
        if (Files.exists(metaFile)) {
            Files.delete(metaFile)
        }
        val latestFile = objectDir.resolve(".latest")
        if (Files.exists(latestFile) && Files.readString(latestFile).trim() == resolved.version?.value) {
            Files.delete(latestFile)
        }
        return deleted
    }

    /**
     * 检查对象是否存在
     * Check if object exists
     *
     * @param ref 对象引用
     *            Object reference
     * @return 对象数据文件是否存在
     *         Whether object data file exists
     */
    override suspend fun exists(ref: ObjectRef): Boolean {
        val resolved = resolveReference(ref) ?: return false
        return Files.exists(dataFileOf(resolved))
    }

    /**
     * 解析对象引用
     * Resolve object reference
     *
     * 如果引用未指定版本，从 .latest 文件读取最新版本号。
     * If reference doesn't specify version, reads latest version from .latest file.
     *
     * @param ref 对象引用
     *            Object reference
     * @return 包含完整版本信息的对象引用，解析失败返回null
     *         Object reference with complete version info, or null if resolution fails
     */
    private fun resolveReference(ref: ObjectRef): ObjectRef? {
        if (ref.version != null) {
            return ref
        }
        val latestFile = resolveObjectDir(ref.path.value).resolve(".latest")
        if (!Files.exists(latestFile)) {
            return null
        }
        val version = Files.readString(latestFile).trim()
        if (version.isEmpty()) {
            return null
        }
        return ref.copy(version = version.let { fuookami.ospf.framework.remote_solver.protocol.domain.ObjectVersion.of(it) })
    }

    /**
     * 获取对象数据文件路径
     * Get object data file path
     *
     * @param ref 对象引用
     *            Object reference
     * @return 数据文件的完整路径
     *         Full path to data file
     */
    private fun dataFileOf(ref: ObjectRef): Path = resolveObjectDir(ref.path.value).resolve("${ref.version?.value}.data")

    /**
     * 获取对象元数据文件路径
     * Get object metadata file path
     *
     * @param ref 对象引用
     *            Object reference
     * @return 元数据文件的完整路径
     *         Full path to metadata file
     */
    private fun metadataFileOf(ref: ObjectRef): Path = resolveObjectDir(ref.path.value).resolve("${ref.version?.value}.meta")

    /**
     * 解析对象目录路径
     * Resolve object directory path
     *
     * 将相对路径转换为绝对路径，同时进行安全检查防止路径遍历攻击。
     * Converts relative path to absolute path with security checks against path traversal attacks.
     *
     * @param path 对象的相对路径
     *             Object relative path
     * @return 对象目录的完整路径
     *         Full path to object directory
     * @throws IllegalArgumentException 如果路径是绝对路径或包含路径遍历字符
     *         IllegalArgumentException if path is absolute or contains traversal characters
     */
    private fun resolveObjectDir(path: String): Path {
        val relative = Paths.get(path).normalize()
        require(!relative.isAbsolute) { "Absolute path is not allowed: $path" }
        require(!relative.startsWith("..")) { "Path traversal is not allowed: $path" }
        val resolved = root.resolve(relative).normalize()
        require(resolved.startsWith(root)) { "Path traversal is not allowed: $path" }
        return resolved
    }
}
