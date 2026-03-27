/**
 * 错误码与异常定义
 * Error codes and exception definitions
 *
 * 定义远程求解器中所有标准化的错误码和异常类型。
 * Defines all standardized error codes and exception types for the remote solver.
 */
package fuookami.ospf.framework.remote_solver.protocol.domain

/**
 * 远程求解器错误码枚举
 * Remote solver error code enumeration
 *
 * 用于标识系统中各类错误场景，便于错误追踪和处理。
 * Used to identify various error scenarios in the system for error tracking and handling.
 */
enum class RemoteSolverErrorCode {
    /** 参数无效 / Invalid argument */
    INVALID_ARGUMENT,

    /** 任务状态转换无效 / Invalid task state transition */
    INVALID_TASK_STATE_TRANSITION,

    /** 无符合条件的节点可用 / No eligible node available */
    NO_ELIGIBLE_NODE_AVAILABLE,

    /** 节点已离线 / Node is offline */
    NODE_OFFLINE,

    /** 求解器执行失败 / Solver execution failed */
    SOLVER_EXECUTION_FAILED,

    /** 检查点导出失败 / Checkpoint export failed */
    CHECKPOINT_EXPORT_FAILED,

    /** 事件发布失败 / Event publish failed */
    EVENT_PUBLISH_FAILED,

    /** 存储 IO 操作失败 / Storage I/O operation failed */
    STORAGE_IO_FAILED,

    /** 任务在最大轮次内未达到终态 / Task did not reach terminal state within max rounds */
    TASK_NOT_TERMINAL_WITHIN_MAX_ROUNDS,

    /** 无兼容节点可用 / No compatible node available */
    NO_COMPATIBLE_NODE_AVAILABLE,

    /** 任务失败（通用） / Task failed (generic) */
    TASK_FAILED,

    /** 任务因硬超时失败 / Task failed due to hard timeout */
    TASK_FAILED_HARD_TIMEOUT,

    /** 任务因切片超时失败 / Task failed due to slice timeout */
    TASK_FAILED_SLICE_TIMEOUT,

    /** 任务因预算超限失败 / Task failed due to budget exceeded */
    TASK_FAILED_BUDGET_EXCEEDED,

    /** 远程求解在最大轮次内未完成 / Remote solve did not complete within max rounds */
    REMOTE_SOLVE_NOT_COMPLETED_WITHIN_MAX_ROUNDS,

    /** 内部错误 / Internal error */
    INTERNAL_ERROR
}

/**
 * 远程求解器异常类
 * Remote solver exception class
 *
 * 封装错误码、消息和元数据的标准化异常类型。
 * Standardized exception type encapsulating error code, message, and metadata.
 *
 * @param code 错误码 / Error code
 * @param message 错误消息 / Error message
 * @param metadata 元数据（用于调试） / Metadata (for debugging)
 * @param cause 原始异常 / Original exception
 */
class RemoteSolverException(
    val code: RemoteSolverErrorCode,
    override val message: String,
    val metadata: Map<String, String> = emptyMap(),
    cause: Throwable? = null
) : RuntimeException(message, cause)

/**
 * 远程求解器错误映射器
 * Remote solver error mapper
 *
 * 将任意异常标准化为 RemoteSolverException，便于统一处理。
 * Normalizes any exception to RemoteSolverException for unified handling.
 */
object RemoteSolverErrorMapper {
    /**
     * 标准化异常
     * Normalize exception
     *
     * 将任意 Throwable 转换为 RemoteSolverException。
     * Converts any Throwable to RemoteSolverException.
     *
     * @param throwable 原始异常 / Original exception
     * @return 标准化的异常 / Normalized exception
     */
    fun normalize(throwable: Throwable): RemoteSolverException {
        if (throwable is RemoteSolverException) {
            return throwable
        }
        if (throwable is IllegalArgumentException) {
            return RemoteSolverException(
                code = RemoteSolverErrorCode.INVALID_ARGUMENT,
                message = throwable.message ?: "Invalid argument",
                cause = throwable
            )
        }
        return RemoteSolverException(
            code = RemoteSolverErrorCode.INTERNAL_ERROR,
            message = throwable.message ?: "Internal error",
            cause = throwable
        )
    }

    /**
     * 获取 API 错误码
     * Get API error code
     *
     * 从异常中提取标准化的 API 错误码字符串。
     * Extracts standardized API error code string from exception.
     *
     * @param throwable 异常 / Exception
     * @return 错误码字符串 / Error code string
     */
    fun apiCodeOf(throwable: Throwable): String = normalize(throwable).code.name

    /**
     * 获取原因码
     * Get reason code
     *
     * 将错误码转换为字符串。
     * Converts error code to string.
     *
     * @param code 错误码 / Error code
     * @return 原因码字符串 / Reason code string
     */
    fun reasonCodeOf(code: RemoteSolverErrorCode): String = code.name
}