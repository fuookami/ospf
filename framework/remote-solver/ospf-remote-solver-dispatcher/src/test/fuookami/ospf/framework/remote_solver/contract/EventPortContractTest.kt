package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.domain.EventRecord
import fuookami.ospf.framework.remote_solver.domain.EventEnvelopeHeaders
import fuookami.ospf.framework.remote_solver.domain.EventSchema
import fuookami.ospf.framework.remote_solver.port.EventPort
import kotlin.math.abs
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

abstract class EventPortContractTest {
    protected abstract fun createFixture(): TestFixture<EventPort>

    @Test
    fun subscribeShouldReceivePublishedEvent() {
        val fixture = createFixture()
        try {
            runSuspend {
                val received = mutableListOf<EventRecord>()
                fixture.subject.subscribe("topic-a", "group-1") { record ->
                    received.add(record)
                }

                val published = fixture.subject.publish("topic-a", "key-a", "hello".toByteArray())
                assertNotNull(published.eventId)
                assertEquals(1, received.size)
                assertEquals("topic-a", received[0].topic)
                assertEquals("key-a", received[0].key)
                assertEquals("hello", received[0].payload.decodeToString())
            }
        } finally {
            fixture.close()
        }
    }

    @Test
    fun unsubscribeShouldStopReceivingEvents() {
        val fixture = createFixture()
        try {
            runSuspend {
                var count = 0
                val subscriptionId = fixture.subject.subscribe("topic-b", "group-1") {
                    count += 1
                }
                fixture.subject.publish("topic-b", "k1", "1".toByteArray())

                val removed = fixture.subject.unsubscribe(subscriptionId)
                assertTrue(removed)
                fixture.subject.publish("topic-b", "k2", "2".toByteArray())
                assertEquals(1, count)
            }
        } finally {
            fixture.close()
        }
    }

    @Test
    fun nackShouldTriggerRetryDelivery() {
        val fixture = createFixture()
        try {
            runSuspend {
                val attempts = mutableListOf<Int>()
                fixture.subject.subscribe("topic-c", "group-1") { record ->
                    attempts.add(record.deliveryAttempt)
                    if (record.deliveryAttempt == 0) {
                        fixture.subject.nack(record)
                    }
                }

                fixture.subject.publish("topic-c", "k", "retry".toByteArray())
                assertEquals(2, attempts.size)
                assertEquals(listOf(0, 1), attempts)
            }
        } finally {
            fixture.close()
        }
    }

    @Test
    fun publishShouldDeduplicateByIdempotencyKey() {
        val fixture = createFixture()
        try {
            runSuspend {
                var delivered = 0
                var deliveredPayload = ""
                fixture.subject.subscribe("topic-d", "group-1") {
                    delivered += 1
                    deliveredPayload = it.payload.decodeToString()
                }

                val first = fixture.subject.publish(
                    topic = "topic-d",
                    key = "task-1",
                    payload = "payload-v1".toByteArray(),
                    headers = mapOf("idempotencyKey" to "dispatch-1")
                )
                val second = fixture.subject.publish(
                    topic = "topic-d",
                    key = "task-1",
                    payload = "payload-v2".toByteArray(),
                    headers = mapOf("idempotencyKey" to "dispatch-1")
                )

                assertEquals(first.eventId, second.eventId)
                assertEquals(1, delivered)
                assertEquals("payload-v1", deliveredPayload)
            }
        } finally {
            fixture.close()
        }
    }

    @Test
    fun publishShouldInjectSchemaVersionHeader() {
        val fixture = createFixture()
        try {
            runSuspend {
                val record = fixture.subject.publish("topic-e", "k", "v".toByteArray())
                assertEquals(EventSchema.CURRENT_VERSION, record.schemaVersion)
                assertEquals(
                    EventSchema.CURRENT_VERSION.toString(),
                    record.headers[EventSchema.HEADER_NAME]
                )
            }
        } finally {
            fixture.close()
        }
    }

    @Test
    fun publishShouldRejectInvalidSchemaVersionHeader() {
        val fixture = createFixture()
        try {
            runSuspend {
                assertFailsWith<IllegalArgumentException> {
                    fixture.subject.publish(
                        topic = "topic-e-invalid",
                        key = "k",
                        payload = "v".toByteArray(),
                        headers = mapOf(EventSchema.HEADER_NAME to "abc")
                    )
                }
                assertFailsWith<IllegalArgumentException> {
                    fixture.subject.publish(
                        topic = "topic-e-invalid",
                        key = "k",
                        payload = "v".toByteArray(),
                        headers = mapOf(EventSchema.HEADER_NAME to "0")
                    )
                }
                assertFailsWith<IllegalArgumentException> {
                    fixture.subject.publish(
                        topic = "topic-e-invalid",
                        key = "k",
                        payload = "v".toByteArray(),
                        headers = mapOf(EventSchema.HEADER_NAME to (EventSchema.CURRENT_VERSION + 1).toString())
                    )
                }
            }
        } finally {
            fixture.close()
        }
    }

    @Test
    fun publishShouldInjectEnvelopeHeaders() {
        val fixture = createFixture()
        try {
            runSuspend {
                val record = fixture.subject.publish("topic-envelope", "k", "v".toByteArray())
                assertEquals("topic-envelope", record.headers[EventEnvelopeHeaders.EVENT_TYPE])
                assertNotNull(record.headers[EventEnvelopeHeaders.PRODUCER])
                assertTrue(record.headers[EventEnvelopeHeaders.PRODUCER]!!.isNotBlank())
                assertEquals(
                    record.createdAtEpochMs.toString(),
                    record.headers[EventEnvelopeHeaders.OCCURRED_AT_EPOCH_MS]
                )
                assertEquals("default", record.headers[EventEnvelopeHeaders.TENANT_ID])
            }
        } finally {
            fixture.close()
        }
    }

    @Test
    fun publishShouldRejectInvalidOccurredAtEpochMsHeader() {
        val fixture = createFixture()
        try {
            runSuspend {
                assertFailsWith<IllegalArgumentException> {
                    fixture.subject.publish(
                        topic = "topic-occurred-at-invalid",
                        key = "k",
                        payload = "v".toByteArray(),
                        headers = mapOf(EventEnvelopeHeaders.OCCURRED_AT_EPOCH_MS to "abc")
                    )
                }
                assertFailsWith<IllegalArgumentException> {
                    fixture.subject.publish(
                        topic = "topic-occurred-at-invalid",
                        key = "k",
                        payload = "v".toByteArray(),
                        headers = mapOf(EventEnvelopeHeaders.OCCURRED_AT_EPOCH_MS to "0")
                    )
                }
            }
        } finally {
            fixture.close()
        }
    }

    @Test
    fun publishShouldBroadcastToDifferentConsumerGroups() {
        val fixture = createFixture()
        try {
            runSuspend {
                var group1Count = 0
                var group2Count = 0
                fixture.subject.subscribe("topic-f", "group-1") {
                    group1Count += 1
                }
                fixture.subject.subscribe("topic-f", "group-2") {
                    group2Count += 1
                }

                fixture.subject.publish("topic-f", "k", "v".toByteArray())
                assertEquals(1, group1Count)
                assertEquals(1, group2Count)
            }
        } finally {
            fixture.close()
        }
    }

    @Test
    fun publishShouldLoadBalanceWithinSameConsumerGroup() {
        val fixture = createFixture()
        try {
            runSuspend {
                var consumer1Count = 0
                var consumer2Count = 0
                fixture.subject.subscribe("topic-g", "group-1") {
                    consumer1Count += 1
                }
                fixture.subject.subscribe("topic-g", "group-1") {
                    consumer2Count += 1
                }

                repeat(5) { index ->
                    fixture.subject.publish("topic-g", "k-$index", "v".toByteArray())
                }

                assertEquals(5, consumer1Count + consumer2Count)
                assertTrue(consumer1Count > 0)
                assertTrue(consumer2Count > 0)
                assertTrue(abs(consumer1Count - consumer2Count) <= 1)
            }
        } finally {
            fixture.close()
        }
    }

    @Test
    fun nackWithFutureRetryAtShouldDelayDelivery() {
        val fixture = createFixture()
        try {
            runSuspend {
                val attempts = mutableListOf<Long>()
                fixture.subject.subscribe("topic-h", "group-1") { record ->
                    attempts.add(System.currentTimeMillis())
                    if (record.deliveryAttempt == 0) {
                        fixture.subject.nack(record, System.currentTimeMillis() + 60L)
                    }
                }

                fixture.subject.publish("topic-h", "k", "retry-delay".toByteArray())
                waitUntil(timeoutMs = 1000L) { attempts.size == 2 }

                assertEquals(2, attempts.size)
                val elapsedMs = attempts[1] - attempts[0]
                assertTrue(elapsedMs >= 40L)
            }
        } finally {
            fixture.close()
        }
    }

    private fun waitUntil(timeoutMs: Long, pollMs: Long = 10L, predicate: () -> Boolean) {
        val deadline = System.currentTimeMillis() + timeoutMs
        while (System.currentTimeMillis() <= deadline) {
            if (predicate()) {
                return
            }
            Thread.sleep(pollMs)
        }
        assertTrue(predicate())
    }
}
