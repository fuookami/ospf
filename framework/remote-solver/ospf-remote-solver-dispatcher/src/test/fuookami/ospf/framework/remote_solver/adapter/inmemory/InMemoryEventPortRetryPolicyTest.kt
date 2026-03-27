package fuookami.ospf.framework.remote_solver.adapter.inmemory

import fuookami.ospf.framework.remote_solver.adapter.infrastructure.SystemClockPort
import fuookami.ospf.framework.remote_solver.adapter.infrastructure.UUIDIdGeneratorPort
import fuookami.ospf.framework.remote_solver.contract.runSuspend
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class InMemoryEventPortRetryPolicyTest {
    @Test
    fun retryPolicyShouldBeConstructedFromProperties() {
        val policy = InMemoryEventRetryPolicy.fromProperties(
            mapOf(
                "event.retry.max-attempts" to "5",
                "event.retry.base-delay-ms" to "40",
                "event.retry.max-delay-ms" to "500"
            )
        )

        assertEquals(5, policy.maxAttempts)
        assertEquals(40L, policy.computeDelayMs(1))
        assertEquals(80L, policy.computeDelayMs(2))
        assertEquals(160L, policy.computeDelayMs(3))
        assertEquals(320L, policy.computeDelayMs(4))
        assertEquals(500L, policy.computeDelayMs(5))
    }

    @Test
    fun retryPolicyShouldComputeExpectedDelays() {
        val immediate = InMemoryEventRetryPolicy(baseDelayMs = 1L)
        assertEquals(1L, immediate.computeDelayMs(1))

        val fixed = InMemoryEventRetryPolicy(
            baseDelayMs = 80L
        )
        assertEquals(80L, fixed.computeDelayMs(1))
        assertEquals(320L, fixed.computeDelayMs(3))

        val exponential = InMemoryEventRetryPolicy(
            baseDelayMs = 50L,
            maxDelayMs = 500L
        )
        assertEquals(50L, exponential.computeDelayMs(1))
        assertEquals(100L, exponential.computeDelayMs(2))
        assertEquals(200L, exponential.computeDelayMs(3))
        assertEquals(400L, exponential.computeDelayMs(4))
        assertEquals(500L, exponential.computeDelayMs(5))
    }

    @Test
    fun nackShouldUsePolicyDelayWhenRetryAtIsNull() {
        val eventPort = InMemoryEventPort(
            clock = SystemClockPort(),
            idGenerator = UUIDIdGeneratorPort(),
            retryPolicy = InMemoryEventRetryPolicy(
                baseDelayMs = 70L
            )
        )

        runSuspend {
            val attempts = mutableListOf<Long>()
            eventPort.subscribe("topic-policy", "group-1") { record ->
                attempts.add(System.currentTimeMillis())
                if (record.deliveryAttempt == 0) {
                    eventPort.nack(record)
                }
            }

            eventPort.publish("topic-policy", "k", "v".toByteArray())
            waitUntil(1000L) { attempts.size == 2 }

            val elapsed = attempts[1] - attempts[0]
            assertTrue(elapsed >= 45L)
        }
    }

    @Test
    fun retryAtShouldOverridePolicyDelay() {
        val eventPort = InMemoryEventPort(
            clock = SystemClockPort(),
            idGenerator = UUIDIdGeneratorPort(),
            retryPolicy = InMemoryEventRetryPolicy(
                baseDelayMs = 800L
            )
        )

        runSuspend {
            val attempts = mutableListOf<Long>()
            eventPort.subscribe("topic-override", "group-1") { record ->
                attempts.add(System.currentTimeMillis())
                if (record.deliveryAttempt == 0) {
                    eventPort.nack(record, System.currentTimeMillis() + 60L)
                }
            }

            eventPort.publish("topic-override", "k", "v".toByteArray())
            waitUntil(1000L) { attempts.size == 2 }

            val elapsed = attempts[1] - attempts[0]
            assertTrue(elapsed >= 40L)
            assertTrue(elapsed < 400L)
        }
    }

    private fun waitUntil(timeoutMs: Long, pollMs: Long = 10L, condition: () -> Boolean) {
        val deadline = System.currentTimeMillis() + timeoutMs
        while (System.currentTimeMillis() <= deadline) {
            if (condition()) {
                return
            }
            Thread.sleep(pollMs)
        }
        assertTrue(condition())
    }
}
