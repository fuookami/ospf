package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.port.DistributedLockPort
import kotlin.test.Test
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

abstract class DistributedLockPortContractTest {
    protected abstract fun createFixture(): TestFixture<DistributedLockPort>

    @Test
    fun acquireAndReleaseLock() {
        val fixture = createFixture()
        try {
            runSuspend {
                val lease1 = fixture.subject.acquire("lock-a", "owner-1", ttlMs = 30000L)
                assertNotNull(lease1)

                val lease2 = fixture.subject.acquire("lock-a", "owner-2", ttlMs = 30000L)
                assertNull(lease2)

                val released = fixture.subject.release(lease1)
                assertTrue(released)

                val lease3 = fixture.subject.acquire("lock-a", "owner-2", ttlMs = 30000L)
                assertNotNull(lease3)
            }
        } finally {
            fixture.close()
        }
    }

    @Test
    fun renewWithOwnerMismatchShouldFail() {
        val fixture = createFixture()
        try {
            runSuspend {
                val lease = fixture.subject.acquire("lock-b", "owner-1", ttlMs = 30000L)
                assertNotNull(lease)

                val fakeLease = lease.copy(owner = "owner-2")
                val renewed = fixture.subject.renew(fakeLease, ttlMs = 30000L)
                assertNull(renewed)
            }
        } finally {
            fixture.close()
        }
    }

    @Test
    fun expiredLeaseShouldAllowReacquireByAnotherOwner() {
        val fixture = createFixture()
        try {
            runSuspend {
                val lease1 = fixture.subject.acquire("lock-c", "owner-1", ttlMs = 40L)
                assertNotNull(lease1)

                Thread.sleep(80L)

                val lease2 = fixture.subject.acquire("lock-c", "owner-2", ttlMs = 30000L)
                assertNotNull(lease2)
                assertTrue(lease2.owner == "owner-2")
                assertTrue(lease2.leaseId != lease1.leaseId)
            }
        } finally {
            fixture.close()
        }
    }

    @Test
    fun nonPositiveTtlShouldFailForAcquireAndRenew() {
        val fixture = createFixture()
        try {
            runSuspend {
                val invalidAcquire = fixture.subject.acquire("lock-d", "owner-1", ttlMs = 0L)
                assertNull(invalidAcquire)

                val lease = fixture.subject.acquire("lock-d", "owner-1", ttlMs = 30000L)
                assertNotNull(lease)

                val invalidRenew = fixture.subject.renew(lease, ttlMs = 0L)
                assertNull(invalidRenew)
            }
        } finally {
            fixture.close()
        }
    }
}
