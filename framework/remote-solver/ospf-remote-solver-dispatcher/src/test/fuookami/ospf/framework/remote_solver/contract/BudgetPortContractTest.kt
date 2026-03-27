package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.port.BudgetPort
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

abstract class BudgetPortContractTest {
    protected abstract fun createFixture(): TestFixture<BudgetPort>

    @Test
    fun configureAndSnapshot() {
        val fixture = createFixture()
        try {
            runSuspend {
                fixture.subject.configureBudget("scope-a", 100.0)

                val snapshot = fixture.subject.snapshot("scope-a")
                assertNotNull(snapshot)
                assertEquals(100.0, snapshot.limit)
                assertEquals(0.0, snapshot.reserved)
                assertEquals(0.0, snapshot.consumed)
            }
        } finally {
            fixture.close()
        }
    }

    @Test
    fun reserveCommitAndRefundFlow() {
        val fixture = createFixture()
        try {
            runSuspend {
                fixture.subject.configureBudget("scope-b", 50.0)

                val reserved = fixture.subject.reserve("scope-b", 10.0)
                assertTrue(reserved)

                val committed = fixture.subject.commit("scope-b", 6.0)
                assertTrue(committed)

                val snapshotAfterCommit = fixture.subject.snapshot("scope-b")
                assertNotNull(snapshotAfterCommit)
                assertEquals(4.0, snapshotAfterCommit.reserved)
                assertEquals(6.0, snapshotAfterCommit.consumed)

                val refunded = fixture.subject.refund("scope-b", 2.0)
                assertTrue(refunded)
                val snapshotAfterRefund = fixture.subject.snapshot("scope-b")
                assertNotNull(snapshotAfterRefund)
                assertEquals(4.0, snapshotAfterRefund.reserved)
                assertEquals(4.0, snapshotAfterRefund.consumed)
            }
        } finally {
            fixture.close()
        }
    }

    @Test
    fun overLimitAndNegativeShouldFail() {
        val fixture = createFixture()
        try {
            runSuspend {
                fixture.subject.configureBudget("scope-c", 5.0)

                val overReserve = fixture.subject.reserve("scope-c", 10.0)
                assertFalse(overReserve)

                val negativeReserve = fixture.subject.reserve("scope-c", -1.0)
                val negativeCommit = fixture.subject.commit("scope-c", -1.0)
                val negativeRefund = fixture.subject.refund("scope-c", -1.0)
                assertFalse(negativeReserve)
                assertFalse(negativeCommit)
                assertFalse(negativeRefund)
            }
        } finally {
            fixture.close()
        }
    }
}
