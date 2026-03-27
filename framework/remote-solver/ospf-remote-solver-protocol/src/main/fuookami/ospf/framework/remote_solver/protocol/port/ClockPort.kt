/**
 * 时钟端口
 * Clock port
 *
 * 提供时间获取的抽象接口，便于测试时模拟时间。
 * Provides abstract interface for time acquisition, allowing time simulation in tests.
 */
@file:OptIn(kotlin.time.ExperimentalTime::class)
package fuookami.ospf.framework.remote_solver.protocol.port

import kotlin.time.Instant

/**
 * 时钟端口接口
 * Clock port interface
 */
interface ClockPort {
    /**
     * 获取当前时间。
     * Get current time.
     *
     * @return 当前时间 / Current time
     */
    fun now(): Instant

    /**
     * 获取当前时间戳（毫秒）
     * Get current timestamp in milliseconds.
     */
    fun nowEpochMs(): Long = now().toEpochMilliseconds()
}
