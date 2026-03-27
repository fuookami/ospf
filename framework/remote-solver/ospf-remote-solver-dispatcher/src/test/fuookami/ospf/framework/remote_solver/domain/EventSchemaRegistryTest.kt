package fuookami.ospf.framework.remote_solver.domain

import kotlin.test.Test
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

class EventSchemaRegistryTest {
    private val registry = EventSchemaRegistry.default()

    @Test
    fun strictModeShouldRejectUnknownEventType() {
        val headers = mapOf(
            EventSchema.HEADER_NAME to EventSchema.CURRENT_VERSION.toString(),
            EventEnvelopeHeaders.EVENT_TYPE to "UnknownTopic",
            EventEnvelopeHeaders.PRODUCER to "test-producer",
            EventEnvelopeHeaders.OCCURRED_AT_EPOCH_MS to "1"
        )
        assertFailsWith<IllegalArgumentException> {
            registry.validateForPublish(
                topic = "UnknownTopic",
                headers = headers,
                schemaVersion = EventSchema.CURRENT_VERSION,
                mode = EventSchemaValidationMode.STRICT
            )
        }
    }

    @Test
    fun lenientModeShouldAllowUnknownEventType() {
        val headers = mapOf(
            EventSchema.HEADER_NAME to EventSchema.CURRENT_VERSION.toString(),
            EventEnvelopeHeaders.EVENT_TYPE to "UnknownTopic",
            EventEnvelopeHeaders.PRODUCER to "test-producer",
            EventEnvelopeHeaders.OCCURRED_AT_EPOCH_MS to "1"
        )
        registry.validateForPublish(
            topic = "UnknownTopic",
            headers = headers,
            schemaVersion = EventSchema.CURRENT_VERSION,
            mode = EventSchemaValidationMode.LENIENT
        )
    }

    @Test
    fun strictModeShouldRejectMissingRequiredHeader() {
        val headers = mapOf(
            EventSchema.HEADER_NAME to EventSchema.CURRENT_VERSION.toString(),
            EventEnvelopeHeaders.EVENT_TYPE to EventTopics.SOLVING_REQUEST,
            EventEnvelopeHeaders.PRODUCER to "test-producer"
        )
        assertFailsWith<IllegalArgumentException> {
            registry.validateForConsume(
                topic = EventTopics.SOLVING_REQUEST,
                headers = headers,
                schemaVersion = EventSchema.CURRENT_VERSION,
                mode = EventSchemaValidationMode.STRICT
            )
        }
    }

    @Test
    fun strictModeShouldAllowMonitorAlertTopic() {
        val headers = mapOf(
            EventSchema.HEADER_NAME to EventSchema.CURRENT_VERSION.toString(),
            EventEnvelopeHeaders.EVENT_TYPE to EventTopics.MONITOR_ALERT,
            EventEnvelopeHeaders.PRODUCER to "test-producer",
            EventEnvelopeHeaders.OCCURRED_AT_EPOCH_MS to "1",
            EventEnvelopeHeaders.TENANT_ID to "default"
        )
        val result = runCatching {
            registry.validateForPublish(
                topic = EventTopics.MONITOR_ALERT,
                headers = headers,
                schemaVersion = EventSchema.CURRENT_VERSION,
                mode = EventSchemaValidationMode.STRICT
            )
        }
        assertTrue(result.isSuccess)
    }
}
