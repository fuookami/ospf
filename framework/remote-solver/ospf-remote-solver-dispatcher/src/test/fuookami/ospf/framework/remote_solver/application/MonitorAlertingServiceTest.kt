@file:OptIn(kotlin.time.ExperimentalTime::class)

package fuookami.ospf.framework.remote_solver.application

import fuookami.ospf.framework.remote_solver.bootstrap.InMemoryRemoteSolverBootstrap
import fuookami.ospf.framework.remote_solver.contract.runSuspend
import fuookami.ospf.framework.remote_solver.domain.EventTopics
import fuookami.ospf.framework.remote_solver.domain.NodeCapabilityProfile
import fuookami.ospf.framework.remote_solver.domain.NodeState
import fuookami.ospf.framework.remote_solver.protocol.port.ClockPort
import java.nio.charset.StandardCharsets
import kotlin.time.Instant
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class MonitorAlertingServiceTest {
    @Test
    fun offlineNodeAlertShouldTriggerEscalateAndRecover() {
        val runtime = InMemoryRemoteSolverBootstrap.create(
            config = RemoteSolverConfig(nodeHeartbeatTimeoutMs = 60_000L)
        )
        runSuspend {
            val now = runtime.service.clockPort().nowEpochMs()
            runtime.nodeStatePort.upsertNode(
                NodeState(
                    nodeId = "node-offline",
                    profile = defaultProfile(nodeId = "node-offline"),
                    availableUnits = 2,
                    lastHeartbeatEpochMs = now - 1_000L,
                    online = false
                )
            )
            val captured = mutableListOf<String>()
            runtime.eventPort.subscribe(EventTopics.MONITOR_ALERT, "group-monitor-alert-flow") { record ->
                captured.add(String(record.payload, StandardCharsets.UTF_8))
            }
            val service = MonitorAlertingService(
                apiFacade = runtime.apiFacade,
                eventPort = runtime.eventPort,
                clock = runtime.service.clockPort(),
                config = MonitorAlertingConfig(
                    enabled = true,
                    checkIntervalMs = 0L,
                    cooldownMs = 0L,
                    escalateAfterConsecutive = 2,
                    staleNodesThreshold = 0,
                    offlineNodesThreshold = 1,
                    failedTasksThreshold = 0,
                    queueDepthThreshold = 0,
                    routeEventEnabled = true,
                    routeWebhookEnabled = false
                )
            )

            service.processNow()
            service.processNow()

            val offlineNode = runtime.nodeStatePort.getNode("node-offline")
            assertTrue(offlineNode != null)
            runtime.nodeStatePort.upsertNode(
                offlineNode.copy(
                    online = true,
                    lastHeartbeat = Instant.fromEpochMilliseconds(now + 60_000L)
                )
            )
            service.processNow()

            assertEquals(3, captured.size)
            assertTrue(captured[0].contains("\"ruleKey\":\"offline_nodes\""))
            assertTrue(captured[0].contains("\"state\":\"TRIGGERED\""))
            assertTrue(captured[1].contains("\"state\":\"ESCALATED\""))
            assertTrue(captured[2].contains("\"state\":\"RECOVERED\""))
        }
    }

    @Test
    fun checkIntervalShouldThrottleAlertEvaluation() {
        val runtime = InMemoryRemoteSolverBootstrap.create(
            config = RemoteSolverConfig(nodeHeartbeatTimeoutMs = 60_000L)
        )
        runSuspend {
            val now = runtime.service.clockPort().nowEpochMs()
            runtime.nodeStatePort.upsertNode(
                NodeState(
                    nodeId = "node-offline-throttle",
                    profile = defaultProfile(nodeId = "node-offline-throttle"),
                    availableUnits = 2,
                    lastHeartbeatEpochMs = now - 1_000L,
                    online = false
                )
            )
            val captured = mutableListOf<String>()
            runtime.eventPort.subscribe(EventTopics.MONITOR_ALERT, "group-monitor-alert-throttle") { record ->
                captured.add(String(record.payload, StandardCharsets.UTF_8))
            }
            val clock = MutableClockPort(1_000L)
            val service = MonitorAlertingService(
                apiFacade = runtime.apiFacade,
                eventPort = runtime.eventPort,
                clock = clock,
                config = MonitorAlertingConfig(
                    enabled = true,
                    checkIntervalMs = 2_000L,
                    cooldownMs = 0L,
                    escalateAfterConsecutive = 2,
                    staleNodesThreshold = 0,
                    offlineNodesThreshold = 1,
                    failedTasksThreshold = 0,
                    queueDepthThreshold = 0,
                    routeEventEnabled = true,
                    routeWebhookEnabled = false
                )
            )

            service.processIfDue()
            service.processIfDue()
            clock.advanceBy(2_000L)
            service.processIfDue()

            assertEquals(2, captured.size)
            assertTrue(captured[0].contains("\"state\":\"TRIGGERED\""))
            assertTrue(captured[1].contains("\"state\":\"ESCALATED\""))
        }
    }

    @Test
    fun webhookRouteShouldReceiveTriggeredAlertWhenEnabled() {
        val runtime = InMemoryRemoteSolverBootstrap.create(
            config = RemoteSolverConfig(nodeHeartbeatTimeoutMs = 60_000L)
        )
        runSuspend {
            val now = runtime.service.clockPort().nowEpochMs()
            runtime.nodeStatePort.upsertNode(
                NodeState(
                    nodeId = "node-offline-webhook",
                    profile = defaultProfile(nodeId = "node-offline-webhook"),
                    availableUnits = 2,
                    lastHeartbeatEpochMs = now - 1_000L,
                    online = false
                )
            )
            val webhook = RecordingWebhookPublisher()
            val service = MonitorAlertingService(
                apiFacade = runtime.apiFacade,
                eventPort = runtime.eventPort,
                clock = runtime.service.clockPort(),
                config = MonitorAlertingConfig(
                    enabled = true,
                    checkIntervalMs = 0L,
                    cooldownMs = 0L,
                    escalateAfterConsecutive = 2,
                    staleNodesThreshold = 0,
                    offlineNodesThreshold = 1,
                    failedTasksThreshold = 0,
                    queueDepthThreshold = 0,
                    routeEventEnabled = false,
                    routeWebhookEnabled = true
                ),
                webhookPublisher = webhook
            )

            service.processNow()

            assertEquals(1, webhook.notifications.size)
            val notification = webhook.notifications.first()
            assertEquals("offline_nodes", notification.ruleKey)
            assertEquals("TRIGGERED", notification.state)
        }
    }

    private fun defaultProfile(nodeId: String): NodeCapabilityProfile =
        NodeCapabilityProfile(
            nodeId = nodeId,
            solverType = "gurobi",
            performanceScore = 1.0,
            pricePerSecond = 0.1,
            minBillingUnitSeconds = 1,
            supportsInterrupt = true,
            supportsCheckpoint = true,
            supportsWarmStart = true,
            parallelUnits = 2
        )

    private class MutableClockPort(
        private var nowEpochMs: Long
    ) : ClockPort {
        override fun now(): Instant = Instant.fromEpochMilliseconds(nowEpochMs)

        fun advanceBy(deltaMs: Long) {
            nowEpochMs += deltaMs
        }
    }

    private class RecordingWebhookPublisher : MonitorAlertWebhookPublisher {
        val notifications = mutableListOf<MonitorAlertNotification>()

        override suspend fun publish(notification: MonitorAlertNotification) {
            notifications.add(notification)
        }
    }
}
