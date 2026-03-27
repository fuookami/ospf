@file:OptIn(kotlin.time.ExperimentalTime::class)

/*
 * 系统时钟端口适配器
 *
 * System Clock Port Adapter
 *
 * 该模块提供基于系统时钟的时间服务实现，用于获取当前时间的 Unix 毫秒时间戳。
 * This module provides a system clock-based time service implementation for obtaining
 * current Unix timestamp in milliseconds.
 */

package fuookami.ospf.framework.remote_solver.adapter.infrastructure

import kotlin.time.Instant
import fuookami.ospf.framework.remote_solver.protocol.port.ClockPort

/**
 * 系统时钟端口
 *
 * System Clock Port
 *
 * 该类实现了 [ClockPort] 接口，使用 Java 系统时钟 ([System.currentTimeMillis]) 提供
 * 当前时间的毫秒时间戳。这是一个简单的生产环境实现，适用于大多数场景。
 *
 * This class implements [ClockPort] interface, using Java system clock ([System.currentTimeMillis])
 * to provide current timestamp in milliseconds. This is a simple production-ready implementation
 * suitable for most scenarios.
 *
 * 在测试环境中，建议使用可控制的 mock 时钟实现以便于测试时间相关逻辑。
 * In test environments, it's recommended to use a controllable mock clock implementation
 * for testing time-related logic.
 */
class SystemClockPort : ClockPort {

    /**
     * 获取当前 Unix 时间戳（毫秒）
     *
     * Get current Unix timestamp in milliseconds
     *
     * 返回自 1970-01-01T00:00:00Z 至当前的毫秒数。
     * Returns the number of milliseconds since 1970-01-01T00:00:00Z.
     *
     * @return 当前 Unix 毫秒时间戳。
     *         Current Unix timestamp in milliseconds.
     */
    override fun now(): Instant = Instant.fromEpochMilliseconds(System.currentTimeMillis())
}
