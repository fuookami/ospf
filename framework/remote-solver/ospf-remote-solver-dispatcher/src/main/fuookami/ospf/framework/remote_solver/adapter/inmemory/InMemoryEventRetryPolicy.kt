/**
 * 内存事件重试策略
 *
 * 定义事件处理失败时的重试策略配置，包括最大重试次数、基础延迟和最大延迟。
 * 采用指数退避算法计算每次重试的延迟时间。
 *
 * In-memory event retry policy.
 *
 * Defines retry policy configuration for failed event handling, including maximum retry attempts,
 * base delay, and maximum delay. Uses exponential backoff algorithm to calculate delay for each retry.
 */
package fuookami.ospf.framework.remote_solver.adapter.inmemory

/**
 * 内存事件重试策略配置
 *
 * 用于配置事件处理失败后的重试行为，支持指数退避算法。
 *
 * In-memory event retry policy configuration.
 *
 * Configures retry behavior after event processing failures, supporting exponential backoff algorithm.
 *
 * @param maxAttempts 最大重试次数
 *                     Maximum retry attempts
 * @param baseDelayMs 基础延迟时间（毫秒），用于指数退避计算
 *                     Base delay time in milliseconds for exponential backoff calculation
 * @param maxDelayMs 最大延迟时间（毫秒），限制单次重试的最大等待时间
 *                    Maximum delay time in milliseconds, limiting maximum wait time per retry
 */
data class InMemoryEventRetryPolicy(
    val maxAttempts: Int = 3,
    val baseDelayMs: Long = 100L,
    val maxDelayMs: Long = 5000L
) {
    /**
     * 计算指定重试次数的延迟时间
     *
     * 使用指数退避算法计算延迟时间：delay = baseDelayMs * 2^(attempt-1)
     * 结果不会超过 maxDelayMs。
     *
     * Computes delay time for the specified retry attempt.
     *
     * Uses exponential backoff algorithm: delay = baseDelayMs * 2^(attempt-1)
     * The result will not exceed maxDelayMs.
     *
     * @param attempt 重试次数（从1开始）
     *                 Retry attempt number (starting from 1)
     * @return 延迟时间（毫秒），如果attempt <= 0则返回0
     *         Delay time in milliseconds, returns 0 if attempt <= 0
     */
    fun computeDelayMs(attempt: Int): Long {
        if (attempt <= 0) return 0L
        val exponentialDelay = baseDelayMs * (1 shl (attempt - 1))
        return exponentialDelay.coerceAtMost(maxDelayMs)
    }

    companion object {
        /**
         * 从属性配置创建重试策略
         *
         * 支持的属性键：
         * - event.retry.max-attempts: 最大重试次数
         * - event.retry.base-delay-ms: 基础延迟时间
         * - event.retry.max-delay-ms: 最大延迟时间
         *
         * Creates retry policy from properties configuration.
         *
         * Supported property keys:
         * - event.retry.max-attempts: Maximum retry attempts
         * - event.retry.base-delay-ms: Base delay time
         * - event.retry.max-delay-ms: Maximum delay time
         *
         * @param properties 属性配置映射
         *                   Property configuration map
         * @return 配置好的重试策略实例
         *         Configured retry policy instance
         */
        fun fromProperties(properties: Map<String, String>): InMemoryEventRetryPolicy {
            val maxAttempts = properties["event.retry.max-attempts"]?.trim()?.toIntOrNull() ?: 3
            val baseDelayMs = properties["event.retry.base-delay-ms"]?.trim()?.toLongOrNull() ?: 100L
            val maxDelayMs = properties["event.retry.max-delay-ms"]?.trim()?.toLongOrNull() ?: 5000L
            return InMemoryEventRetryPolicy(
                maxAttempts = maxAttempts.coerceAtLeast(1),
                baseDelayMs = baseDelayMs.coerceAtLeast(1L),
                maxDelayMs = maxDelayMs.coerceAtLeast(baseDelayMs)
            )
        }
    }
}