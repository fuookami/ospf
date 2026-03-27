package fuookami.ospf.framework.remote_solver.adapter.inmemory

import fuookami.ospf.framework.remote_solver.adapter.infrastructure.SystemClockPort
import fuookami.ospf.framework.remote_solver.adapter.infrastructure.UUIDIdGeneratorPort
import fuookami.ospf.framework.remote_solver.contract.runSuspend
import fuookami.ospf.framework.remote_solver.domain.EventRecord
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class InMemoryEventPortQueryTaskEventsTest {
    private fun createPort(): InMemoryEventPort = InMemoryEventPort(
        clock = SystemClockPort(),
        idGenerator = UUIDIdGeneratorPort(),
        retryPolicy = InMemoryEventRetryPolicy(baseDelayMs = 0L)
    )

    @Test
    fun queryTaskEventsShouldMatchByEventKey() {
        val port = createPort()
        runSuspend {
            port.publish(
                topic = "test-topic",
                key = "task-123",
                payload = """{"status":"QUEUED"}""".toByteArray(),
                headers = emptyMap()
            )

            val events = port.queryTaskEvents(taskId = "task-123", limit = 10)
            assertEquals(1, events.size)
            assertEquals("task-123", events[0].key)
        }
    }

    @Test
    fun queryTaskEventsShouldMatchByTaskIdHeader() {
        val port = createPort()
        runSuspend {
            port.publish(
                topic = "test-topic",
                key = "slice-456",
                payload = """{"action":"slice-start"}""".toByteArray(),
                headers = mapOf("taskId" to "task-789")
            )

            val events = port.queryTaskEvents(taskId = "task-789", limit = 10)
            assertEquals(1, events.size)
            assertEquals("slice-456", events[0].key)
            assertEquals("task-789", events[0].headers["taskId"])
        }
    }

    @Test
    fun queryTaskEventsShouldMatchByTaskIdInJsonPayload() {
        val port = createPort()
        runSuspend {
            // Publish event with taskId in JSON payload (new format)
            port.publish(
                topic = "test-topic",
                key = "dispatch-001",
                payload = """{"taskId":"task-abc","action":"dispatch"}""".toByteArray(),
                headers = emptyMap()
            )

            val events = port.queryTaskEvents(taskId = "task-abc", limit = 10)
            assertEquals(1, events.size, "Should match taskId in JSON payload")
            assertEquals("dispatch-001", events[0].key)
        }
    }

    @Test
    fun queryTaskEventsShouldMatchAllFormatsForSameTask() {
        val port = createPort()
        runSuspend {
            // Event 1: matched by key
            port.publish(
                topic = "test-topic",
                key = "task-xyz",
                payload = """{"status":"QUEUED"}""".toByteArray(),
                headers = emptyMap()
            )

            // Event 2: matched by header
            port.publish(
                topic = "test-topic",
                key = "slice-xyz-1",
                payload = """{"action":"slice-start"}""".toByteArray(),
                headers = mapOf("taskId" to "task-xyz")
            )

            // Event 3: matched by JSON payload
            port.publish(
                topic = "test-topic",
                key = "dispatch-xyz",
                payload = """{"taskId":"task-xyz","action":"dispatch"}""".toByteArray(),
                headers = emptyMap()
            )

            val events = port.queryTaskEvents(taskId = "task-xyz", limit = 10)
            assertEquals(3, events.size, "Should match all three event formats")
        }
    }

    @Test
    fun queryTaskEventsShouldNotMatchUnrelatedEvents() {
        val port = createPort()
        runSuspend {
            port.publish(
                topic = "test-topic",
                key = "task-111",
                payload = """{"status":"QUEUED"}""".toByteArray(),
                headers = emptyMap()
            )

            port.publish(
                topic = "test-topic",
                key = "task-222",
                payload = """{"status":"QUEUED"}""".toByteArray(),
                headers = emptyMap()
            )

            val events = port.queryTaskEvents(taskId = "task-111", limit = 10)
            assertEquals(1, events.size)
            assertEquals("task-111", events[0].key)
        }
    }

    @Test
    fun queryTaskEventsShouldRespectLimit() {
        val port = createPort()
        runSuspend {
            repeat(10) { i ->
                port.publish(
                    topic = "test-topic",
                    key = "task-limit-$i",
                    payload = """{"taskId":"task-limit","index":$i}""".toByteArray(),
                    headers = emptyMap()
                )
            }

            val events = port.queryTaskEvents(taskId = "task-limit", limit = 5)
            assertEquals(5, events.size, "Should respect limit parameter")
        }
    }
}