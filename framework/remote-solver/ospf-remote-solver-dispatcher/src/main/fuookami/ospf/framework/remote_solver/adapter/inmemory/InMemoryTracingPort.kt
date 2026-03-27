/**
 * 内存追踪端口适配器
 *
 * 提供基于内存的分布式追踪实现，用于测试和非持久化场景。
 * 支持追踪上下文传播、跨度嵌套和追踪/跨度ID生成。
 *
 * In-memory tracing port adapter.
 *
 * Provides memory-based distributed tracing implementation for testing and non-persistent scenarios.
 * Supports trace context propagation, span nesting, and trace/span ID generation.
 */
package fuookami.ospf.framework.remote_solver.adapter.inmemory

import fuookami.ospf.framework.remote_solver.port.TracingPort
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicLong

/**
 * 内存追踪端口实现
 *
 * 使用线程本地存储管理追踪上下文，支持跨度的嵌套和自动传播。
 *
 * In-memory tracing port implementation.
 *
 * Uses thread-local storage for managing trace context, supporting span nesting and automatic propagation.
 */
class InMemoryTracingPort : TracingPort {
    /**
     * 跨度计数器
     *
     * 用于生成唯一的跨度标识符。
     *
     * Span counter.
     *
     * Used for generating unique span identifiers.
     */
    private val spanCounter = AtomicLong(0)

    /**
     * 当前跨度上下文映射
     *
     * 键为线程名称，值为当前活跃的跨度上下文。
     *
     * Current span context map.
     *
     * Keyed by thread name, valued by the currently active span context.
     */
    private val currentSpan = ConcurrentHashMap<String, SpanContext>()

    /**
     * 跨度上下文
     *
     * 存储追踪和跨度标识符。
     *
     * Span context.
     *
     * Stores trace and span identifiers.
     *
     * @param traceId 追踪ID
     *                Trace ID
     * @param spanId 跨度ID
     *               Span ID
     */
    private data class SpanContext(
        val traceId: String,
        val spanId: String
    )

    /**
     * 上下文键
     *
     * 使用当前线程名称作为上下文存储键。
     *
     * Context key.
     *
     * Uses current thread name as context storage key.
     */
    private val contextKey: String
        get() = Thread.currentThread().name

    /**
     * 在追踪跨度中执行代码块
     *
     * 创建新的追踪跨度并在其中执行指定代码块，支持嵌套跨度。
     * 执行完成后自动恢复父跨度上下文。
     *
     * Executes code block in a trace span.
     *
     * Creates a new trace span and executes the specified code block within it, supporting nested spans.
     * Automatically restores parent span context after execution completes.
     *
     * @param name 跨度名称
     *             Span name
     * @param attributes 跨度属性映射
     *                   Span attribute map
     * @param block 要执行的代码块
     *              Code block to execute
     * @return 代码块的执行结果
     *         Execution result of the code block
     */
    override suspend fun <T> inSpan(
        name: String,
        attributes: Map<String, String>,
        block: suspend () -> T
    ): T {
        val parentSpan = currentSpan[contextKey]
        val traceId = parentSpan?.traceId ?: generateTraceId()
        val spanId = generateSpanId()
        val previousSpan = currentSpan.put(contextKey, SpanContext(traceId, spanId))
        try {
            return block()
        } finally {
            if (previousSpan != null) {
                currentSpan[contextKey] = previousSpan
            } else {
                currentSpan.remove(contextKey)
            }
        }
    }

    /**
     * 获取当前追踪ID
     *
     * 返回当前线程活跃跨度所属的追踪ID。
     *
     * Gets current trace ID.
     *
     * Returns the trace ID of the currently active span for the current thread.
     *
     * @return 追踪ID，如果没有活跃跨度则返回null
     *         Trace ID, returns null if no active span
     */
    override fun currentTraceId(): String? = currentSpan[contextKey]?.traceId

    /**
     * 获取当前跨度ID
     *
     * 返回当前线程活跃跨度的ID。
     *
     * Gets current span ID.
     *
     * Returns the ID of the currently active span for the current thread.
     *
     * @return 跨度ID，如果没有活跃跨度则返回null
     *         Span ID, returns null if no active span
     */
    override fun currentSpanId(): String? = currentSpan[contextKey]?.spanId

    /**
     * 生成追踪ID
     *
     * 创建唯一的追踪标识符，包含时间戳和计数器。
     *
     * Generates trace ID.
     *
     * Creates a unique trace identifier containing timestamp and counter.
     *
     * @return 生成的追踪ID字符串
     *         Generated trace ID string
     */
    private fun generateTraceId(): String = "trace-${System.currentTimeMillis()}-${spanCounter.incrementAndGet()}"

    /**
     * 生成跨度ID
     *
     * 创建唯一的跨度标识符。
     *
     * Generates span ID.
     *
     * Creates a unique span identifier.
     *
     * @return 生成的跨度ID字符串
     *         Generated span ID string
     */
    private fun generateSpanId(): String = "span-${spanCounter.incrementAndGet()}"
}