package fuookami.ospf.framework.remote_solver.adapter.inmemory

import fuookami.ospf.framework.remote_solver.adapter.infrastructure.CanonicalMetricsPort
import fuookami.ospf.framework.remote_solver.contract.runSuspend
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull

class CanonicalMetricsPortTest {
    @Test
    fun canonicalMetricsPortShouldMapNamesAndTags() {
        val delegate = InMemoryMetricsPort()
        val metrics = CanonicalMetricsPort(delegate)

        runSuspend {
            metrics.increment("task.failed", tags = mapOf("reason" to "budget"))
            metrics.gauge("slice.cost", 12.5, tags = mapOf("nodeId" to "node-a"))
            metrics.timing("slice.runtime.ms", 1234L, tags = mapOf("nodeId" to "node-a"))
        }

        assertEquals(
            1L,
            delegate.counter("remote_solver_task_failed_total", mapOf("reason" to "budget"))
        )
        val gauge = delegate.gaugeValue("remote_solver_slice_cost", mapOf("node_id" to "node-a"))
        assertNotNull(gauge)
        assertEquals(12.5, gauge)
        assertEquals(
            listOf(1234L),
            delegate.timingValues("remote_solver_slice_runtime_ms", mapOf("node_id" to "node-a"))
        )
    }
}
