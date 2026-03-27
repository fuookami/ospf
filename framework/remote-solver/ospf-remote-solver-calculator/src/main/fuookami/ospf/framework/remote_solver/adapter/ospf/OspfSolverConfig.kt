/**
 * OSPF 求解器配置
 * OSPF solver configuration
 *
 * 定义 OSPF 求解器执行的配置参数。
 * Defines configuration parameters for OSPF solver execution.
 */
package fuookami.ospf.framework.remote_solver.adapter.ospf

/**
 * OSPF 求解器配置
 * Configuration for OSPF solver execution
 *
 * @param timeLimitMs 时间限制（毫秒） / Time limit in milliseconds
 * @param solutionLimit 解数量限制 / Solution count limit
 * @param gapTolerance MIP 间隙容忍度 / MIP gap tolerance
 * @param threads 线程数 / Thread count
 * @param mipFocus MIP 优化焦点 / MIP optimization focus
 * @param outputFlag 输出标志 / Output flag
 * @param logToConsole 是否输出到控制台 / Whether to log to console
 * @param extension 扩展参数 / Extension parameters
 */
data class OspfSolverConfig(
    val timeLimitMs: Long? = null,
    val solutionLimit: Int? = null,
    val gapTolerance: Double? = null,
    val threads: Int? = null,
    val mipFocus: Int? = null,
    val outputFlag: Int? = null,
    val logToConsole: Boolean = false,
    val extension: Map<String, String> = emptyMap()
) {
    companion object {
        /**
         * 从 Map 创建配置
         * Create configuration from map
         */
        fun fromMap(map: Map<String, String>): OspfSolverConfig = OspfSolverConfig(
            timeLimitMs = map["timeLimitMs"]?.toLongOrNull(),
            solutionLimit = map["solutionLimit"]?.toIntOrNull(),
            gapTolerance = map["gapTolerance"]?.toDoubleOrNull(),
            threads = map["threads"]?.toIntOrNull(),
            mipFocus = map["mipFocus"]?.toIntOrNull(),
            outputFlag = map["outputFlag"]?.toIntOrNull(),
            logToConsole = map["logToConsole"]?.toBooleanStrictOrNull() ?: false,
            extension = map.filterKeys { it !in setOf("timeLimitMs", "solutionLimit", "gapTolerance", "threads", "mipFocus", "outputFlag", "logToConsole") }
        )
    }

    /**
     * 转换为 Map
     * Convert to map
     */
    fun toMap(): Map<String, String> = buildMap {
        timeLimitMs?.let { put("timeLimitMs", it.toString()) }
        solutionLimit?.let { put("solutionLimit", it.toString()) }
        gapTolerance?.let { put("gapTolerance", it.toString()) }
        threads?.let { put("threads", it.toString()) }
        mipFocus?.let { put("mipFocus", it.toString()) }
        outputFlag?.let { put("outputFlag", it.toString()) }
        if (logToConsole) put("logToConsole", "true")
        putAll(extension)
    }
}