package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.port.MetricsPort
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull

data class MetricsPortFixture(
    val subject: MetricsPort,
    val counterReader: (name: String, tags: Map<String, String>) -> Long,
    val gaugeReader: (name: String, tags: Map<String, String>) -> Double?,
    val timingReader: (name: String, tags: Map<String, String>) -> List<Long>,
    val close: () -> Unit = {}
)

abstract class MetricsPortContractTest {
    protected abstract fun createFixture(): MetricsPortFixture

    @Test
    fun incrementShouldAccumulateByMetricKey() {
        val fixture = createFixture()
        try {
            runSuspend {
                fixture.subject.increment(
                    name = "task.scheduled",
                    delta = 1,
                    tags = mapOf("nodeId" to "node-a", "topic" to "dispatch")
                )
                fixture.subject.increment(
                    name = "task.scheduled",
                    delta = 2,
                    tags = mapOf("topic" to "dispatch", "nodeId" to "node-a")
                )
            }

            val value = fixture.counterReader(
                "task.scheduled",
                mapOf("nodeId" to "node-a", "topic" to "dispatch")
            )
            assertEquals(3L, value)
        } finally {
            fixture.close()
        }
    }

    @Test
    fun gaugeShouldKeepLatestValue() {
        val fixture = createFixture()
        try {
            runSuspend {
                fixture.subject.gauge("queue.length", 2.0, tags = mapOf("queue" to "complex"))
                fixture.subject.gauge("queue.length", 5.0, tags = mapOf("queue" to "complex"))
            }

            val value = fixture.gaugeReader("queue.length", mapOf("queue" to "complex"))
            assertNotNull(value)
            assertEquals(5.0, value)
        } finally {
            fixture.close()
        }
    }

    @Test
    fun timingShouldAppendSamples() {
        val fixture = createFixture()
        try {
            runSuspend {
                fixture.subject.timing("slice.runtime.ms", 1200L, tags = mapOf("nodeId" to "node-b"))
                fixture.subject.timing("slice.runtime.ms", 800L, tags = mapOf("nodeId" to "node-b"))
            }

            val values = fixture.timingReader("slice.runtime.ms", mapOf("nodeId" to "node-b"))
            assertEquals(listOf(1200L, 800L), values)
        } finally {
            fixture.close()
        }
    }
}
