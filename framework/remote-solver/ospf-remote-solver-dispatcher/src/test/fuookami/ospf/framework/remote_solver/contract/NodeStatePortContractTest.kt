@file:OptIn(kotlin.time.ExperimentalTime::class)

package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.domain.NodeCapabilityProfile
import fuookami.ospf.framework.remote_solver.domain.NodeState
import fuookami.ospf.framework.remote_solver.port.NodeStatePort
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

abstract class NodeStatePortContractTest {
    protected abstract fun createFixture(): TestFixture<NodeStatePort>

    @Test
    fun upsertAndListOnlineNodes() {
        val fixture = createFixture()
        try {
            runSuspend {
                fixture.subject.upsertNode(sampleNode("node-a", online = true))
                fixture.subject.upsertNode(sampleNode("node-b", online = false))

                val all = fixture.subject.listNodes(onlineOnly = false)
                val online = fixture.subject.listNodes(onlineOnly = true)

                assertEquals(2, all.size)
                assertEquals(1, online.size)
                assertEquals("node-a", online[0].nodeId.value)
            }
        } finally {
            fixture.close()
        }
    }

    @Test
    fun occupyAndReleaseUnitShouldRespectBounds() {
        val fixture = createFixture()
        try {
            runSuspend {
                fixture.subject.upsertNode(sampleNode("node-c", units = 1, online = true))

                val first = fixture.subject.occupyUnit("node-c")
                val second = fixture.subject.occupyUnit("node-c")
                assertTrue(first)
                assertFalse(second)

                val release = fixture.subject.releaseUnit("node-c")
                assertTrue(release)
                val node = fixture.subject.getNode("node-c")
                assertNotNull(node)
                assertEquals(1, node.availableUnits)
            }
        } finally {
            fixture.close()
        }
    }

    @Test
    fun heartbeatShouldRefreshTimestampAndOnlineState() {
        val fixture = createFixture()
        try {
            runSuspend {
                fixture.subject.upsertNode(
                    sampleNode("node-d", units = 3, online = false, heartbeatAt = 1000L)
                        .copy(availableUnits = 0)
                )
                fixture.subject.heartbeat("node-d", 5000L)

                val node = fixture.subject.getNode("node-d")
                assertNotNull(node)
                assertTrue(node.online)
                assertEquals(5000L, node.lastHeartbeatEpochMs)
                assertEquals(3, node.availableUnits)
            }
        } finally {
            fixture.close()
        }
    }

    private fun sampleNode(
        nodeId: String,
        units: Int = 2,
        online: Boolean = true,
        heartbeatAt: Long = 1000L
    ): NodeState {
        val profile = NodeCapabilityProfile(
            nodeId = nodeId,
            solverType = "gurobi",
            performanceScore = 1.0,
            pricePerSecond = 0.1,
            minBillingUnitSeconds = 1L,
            supportsInterrupt = true,
            supportsCheckpoint = true,
            supportsWarmStart = true,
            parallelUnits = units
        )
        return NodeState(
            nodeId = nodeId,
            profile = profile,
            availableUnits = units,
            lastHeartbeatEpochMs = heartbeatAt,
            online = online
        )
    }
}
