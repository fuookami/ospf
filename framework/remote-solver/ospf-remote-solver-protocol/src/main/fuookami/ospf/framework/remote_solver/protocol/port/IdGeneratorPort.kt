/**
 * ID 生成器端口
 * ID generator port
 *
 * 提供唯一 ID 生成的抽象接口，便于测试时控制 ID 生成。
 * Provides abstract interface for unique ID generation, allowing ID control in tests.
 */
package fuookami.ospf.framework.remote_solver.protocol.port

import fuookami.ospf.framework.remote_solver.protocol.domain.*

/**
 * ID 生成器端口接口
 * ID generator port interface
 */
interface IdGeneratorPort {
    /**
     * 生成新的唯一 ID
     * Generate new unique ID
     *
     * @param prefix ID 前缀 / ID prefix
     * @return 唯一 ID 字符串 / Unique ID string
     */
    fun newId(prefix: String = ""): String

    /** 生成任务 ID / Generate task ID */
    fun nextTaskId(): TaskId = TaskId.of(newId("task"))

    /** 生成切片 ID / Generate slice ID */
    fun nextSliceId(): SliceId = SliceId.of(newId("slice"))

    /** 生成句柄 ID / Generate handle ID */
    fun nextHandleId(): HandleId = HandleId.of(newId("handle"))

    /** 生成 trace ID / Generate trace ID */
    fun nextTraceId(): TraceId = TraceId.of(newId("trace"))
}
