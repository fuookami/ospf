/*
 * OSPF 模型加载器
 * OSPF Model Loader
 *
 * 从对象存储加载模型数据和求解器配置。
 * Loads model data and solver configuration from object storage.
 * 支持多种模型格式的自动检测。
 * Supports automatic detection of multiple model formats.
 */
package fuookami.ospf.framework.remote_solver.adapter.ospf

import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.port.ObjectStoragePort

/**
 * OSPF 模型加载器
 * OSPF Model Loader
 *
 * 处理从对象存储加载模型数据的操作。
 * Handles loading model data from object storage.
 * 支持多种模型格式（LP、MPS、JSON、AMPL 等）。
 * Supports multiple model formats (LP, MPS, JSON, AMPL, etc).
 *
 * @param objectStoragePort 对象存储端口，用于读取模型文件
 *                          Object storage port for reading model files
 */
class OspfModelLoader(
    private val objectStoragePort: ObjectStoragePort
) {
    /**
     * 从存储加载模型字节
     * Load model bytes from storage
     *
     * 实际的模型格式（LP、MPS、JSON 等）由文件扩展名或元数据决定。
     * Actual model format (LP, MPS, JSON, etc.) is determined by file extension or metadata.
     *
     * @param modelRef 模型的对象引用
     *                  Model object reference
     * @return 模型数据的字节数组，不存在返回 null
     *         Model data byte array, or null if not found
     */
    suspend fun loadModel(modelRef: ObjectRef): ByteArray? {
        return objectStoragePort.get(modelRef)
    }

    /**
     * 从存储加载求解器配置
     * Load solver configuration from storage
     *
     * @param configRef 配置的对象引用
     *                   Config object reference
     * @return 配置数据的字节数组，不存在返回 null
     *         Config data byte array, or null if not found
     */
    suspend fun loadConfig(configRef: ObjectRef): ByteArray? {
        return objectStoragePort.get(configRef)
    }

    /**
     * 加载检查点/快照数据用于热启动
     * Load checkpoint/snapshot data for warm start
     *
     * @param checkpointRef 检查点的对象引用
     *                       Checkpoint object reference
     * @return 检查点数据的字节数组，不存在返回 null
     *         Checkpoint data byte array, or null if not found
     */
    suspend fun loadCheckpoint(checkpointRef: ObjectRef): ByteArray? {
        return objectStoragePort.get(checkpointRef)
    }

    /**
     * 从路径扩展名检测模型格式
     * Detect model format from path extension
     *
     * @param path 文件路径
     *              File path
     * @return 检测到的模型格式
     *         Detected model format
     */
    fun detectModelFormat(path: String): ModelFormat {
        val lower = path.lowercase()
        return when {
            lower.endsWith(".lp") -> ModelFormat.LP
            lower.endsWith(".mps") || lower.endsWith(".mps.gz") -> ModelFormat.MPS
            lower.endsWith(".json") -> ModelFormat.JSON
            lower.endsWith(".dat") -> ModelFormat.DAT
            lower.endsWith(".mod") -> ModelFormat.MOD
            lower.endsWith(".py") -> ModelFormat.PYTHON
            else -> ModelFormat.UNKNOWN
        }
    }
}

/**
 * 模型格式枚举
 * Model Format Enum
 *
 * 定义支持的模型文件格式类型。
 * Defines supported model file format types.
 */
enum class ModelFormat {
    LP,       // CPLEX LP 格式
              // CPLEX LP format
    MPS,      // MPS 格式
              // MPS format
    JSON,     // JSON 格式
              // JSON format
    DAT,      // AMPL 数据格式
              // AMPL data format
    MOD,      // AMPL 模型格式
              // AMPL model format
    PYTHON,   // Python 脚本
              // Python script
    UNKNOWN   // 未知格式
              // Unknown format
}