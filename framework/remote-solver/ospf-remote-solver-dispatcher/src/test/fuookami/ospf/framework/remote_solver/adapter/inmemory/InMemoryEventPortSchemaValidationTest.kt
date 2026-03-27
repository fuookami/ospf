package fuookami.ospf.framework.remote_solver.adapter.inmemory

import fuookami.ospf.framework.remote_solver.adapter.infrastructure.SystemClockPort
import fuookami.ospf.framework.remote_solver.adapter.infrastructure.UUIDIdGeneratorPort
import fuookami.ospf.framework.remote_solver.contract.runSuspend
import fuookami.ospf.framework.remote_solver.domain.EventSchemaValidationMode
import kotlin.test.Test
import kotlin.test.assertFailsWith

class InMemoryEventPortSchemaValidationTest {
    @Test
    fun strictModeShouldRejectUnknownTopicOnPublish() {
        val eventPort = InMemoryEventPort(
            clock = SystemClockPort(),
            idGenerator = UUIDIdGeneratorPort(),
            schemaValidationMode = EventSchemaValidationMode.STRICT
        )
        assertFailsWith<IllegalArgumentException> {
            runSuspend {
                eventPort.publish("topic-unknown", "k", "v".toByteArray())
            }
        }
    }
}
