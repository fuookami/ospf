package fuookami.ospf.framework.remote_solver.adapter.kafka

import fuookami.ospf.framework.remote_solver.adapter.infrastructure.SystemClockPort
import fuookami.ospf.framework.remote_solver.adapter.infrastructure.UUIDIdGeneratorPort
import fuookami.ospf.framework.remote_solver.contract.runSuspend
import kotlin.test.Test
import kotlin.test.assertTrue

class KafkaEventPortQueryTest {
    @Test
    fun queryTaskEventsShouldReturnEmptyWhenNoEventPublished() {
        val port = KafkaEventPort(
            clock = SystemClockPort(),
            idGenerator = UUIDIdGeneratorPort(),
            bootstrapServers = "127.0.0.1:1",
            queryPollTimeoutMs = 100L,
            queryMaxPollRounds = 1
        )
        runSuspend {
            val events = port.queryTaskEvents(taskId = "task-x", limit = 10)
            assertTrue(events.isEmpty())
        }
    }
}
