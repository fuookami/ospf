package fuookami.ospf.framework.remote_solver.adapter.mirroring

import fuookami.ospf.framework.remote_solver.contract.runSuspend
import fuookami.ospf.framework.remote_solver.domain.EventRecord
import fuookami.ospf.framework.remote_solver.port.EventPort
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

class MirroringEventPortTest {
    @Test
    fun publishShouldWriteToPrimaryAndMirror() {
        val primary = RecordingEventPort("primary")
        val mirror = RecordingEventPort("mirror")
        val eventPort = MirroringEventPort(primary = primary, mirror = mirror, failOpen = true)

        val published = runSuspend {
            eventPort.publish("topic-a", "k", "v".toByteArray())
        }

        assertNotNull(published.eventId)
        assertEquals(1, primary.published.size)
        assertEquals(1, mirror.published.size)
        assertEquals("topic-a", primary.published[0].topic)
        assertEquals("topic-a", mirror.published[0].topic)
    }

    @Test
    fun failOpenShouldIgnoreMirrorPublishFailure() {
        val primary = RecordingEventPort("primary")
        val mirror = RecordingEventPort("mirror", failOnPublish = true)
        val eventPort = MirroringEventPort(primary = primary, mirror = mirror, failOpen = true)

        val published = runSuspend {
            eventPort.publish("topic-b", "k", "v".toByteArray())
        }

        assertNotNull(published.eventId)
        assertEquals(1, primary.published.size)
        assertEquals(1, mirror.publishAttemptCount)
    }

    @Test
    fun failClosedShouldPropagateMirrorPublishFailure() {
        val primary = RecordingEventPort("primary")
        val mirror = RecordingEventPort("mirror", failOnPublish = true)
        val eventPort = MirroringEventPort(primary = primary, mirror = mirror, failOpen = false)

        assertFailsWith<IllegalStateException> {
            runSuspend {
                eventPort.publish("topic-c", "k", "v".toByteArray())
            }
        }
        assertEquals(1, primary.published.size)
        assertEquals(1, mirror.publishAttemptCount)
    }

    @Test
    fun subscribeAckNackShouldUsePrimaryPort() {
        val primary = RecordingEventPort("primary")
        val mirror = RecordingEventPort("mirror")
        val eventPort = MirroringEventPort(primary = primary, mirror = mirror, failOpen = true)

        runSuspend {
            val subscriptionId = eventPort.subscribe("topic-d", "group-1") {}
            assertTrue(subscriptionId.startsWith("sub-primary"))
            val record = primary.publish("topic-d", "k", "v".toByteArray())
            eventPort.ack(record)
            eventPort.nack(record)
            eventPort.unsubscribe(subscriptionId)
        }

        assertEquals(1, primary.subscribeCount)
        assertEquals(0, mirror.subscribeCount)
        assertEquals(1, primary.ackCount)
        assertEquals(0, mirror.ackCount)
        assertEquals(1, primary.nackCount)
        assertEquals(0, mirror.nackCount)
    }

    private class RecordingEventPort(
        private val name: String,
        private val failOnPublish: Boolean = false
    ) : EventPort {
        var publishAttemptCount: Int = 0
            private set
        var subscribeCount: Int = 0
            private set
        var ackCount: Int = 0
            private set
        var nackCount: Int = 0
            private set
        private var sequence: Int = 0

        val published = mutableListOf<EventRecord>()

        override suspend fun publish(
            topic: String,
            key: String,
            payload: ByteArray,
            headers: Map<String, String>
        ): EventRecord {
            publishAttemptCount += 1
            if (failOnPublish) {
                throw IllegalStateException("publish failed on $name")
            }
            sequence += 1
            val record = EventRecord(
                eventId = "evt-$name-$sequence",
                topic = topic,
                key = key,
                payload = payload,
                createdAtEpochMs = sequence.toLong(),
                headers = headers
            )
            published.add(record)
            return record
        }

        override suspend fun subscribe(
            topic: String,
            consumerGroup: String,
            handler: suspend (EventRecord) -> Unit
        ): String {
            subscribeCount += 1
            return "sub-$name-$subscribeCount"
        }

        override suspend fun unsubscribe(subscriptionId: String): Boolean = true

        override suspend fun ack(record: EventRecord) {
            ackCount += 1
        }

        override suspend fun nack(record: EventRecord, retryAtEpochMs: Long?) {
            nackCount += 1
        }
    }
}
