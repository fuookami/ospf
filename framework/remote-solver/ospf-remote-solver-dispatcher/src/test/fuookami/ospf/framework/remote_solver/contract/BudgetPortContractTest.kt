package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.port.BudgetPort
import fuookami.ospf.framework.remote_solver.domain.BudgetReservation
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

    @Test
    fun identifiedSettlementIsAtomicAndIdempotent() {
        val fixture = createFixture()
        try {
            runSuspend {
                fixture.subject.configureBudget("scope-settle", 10.0)
                val reservation = BudgetReservation(
                    reservationId = "slice-settle-1",
                    scope = "scope-settle",
                    amount = 8.0
                )
                assertTrue(fixture.subject.reserve(reservation))
                assertTrue(fixture.subject.settle(reservation, actual = 5.0))

                val settled = fixture.subject.snapshot("scope-settle")
                assertNotNull(settled)
                assertEquals(0.0, settled.reserved)
                assertEquals(5.0, settled.consumed)

                // A retry must not charge the same reservation twice.
                assertTrue(fixture.subject.settle(reservation, actual = 5.0))
                val afterRetry = fixture.subject.snapshot("scope-settle")
                assertNotNull(afterRetry)
                assertEquals(settled, afterRetry)
                assertFalse(fixture.subject.settle(reservation, actual = 4.0))
            }
        } finally {
            fixture.close()
        }
    }

    @Test
    fun failedSettlementLeavesReservationAndConsumptionUnchanged() {
        val fixture = createFixture()
        try {
            runSuspend {
                fixture.subject.configureBudget("scope-settle-fail", 10.0)
                val reservation = BudgetReservation(
                    reservationId = "slice-settle-fail",
                    scope = "scope-settle-fail",
                    amount = 8.0
                )
                assertTrue(fixture.subject.reserve(reservation))
                assertFalse(fixture.subject.settle(reservation, actual = 11.0))
                val unchanged = fixture.subject.snapshot("scope-settle-fail")
                assertNotNull(unchanged)
                assertEquals(8.0, unchanged.reserved)
                assertEquals(0.0, unchanged.consumed)
                assertTrue(fixture.subject.settle(reservation, actual = 2.0))
            }
        } finally {
            fixture.close()
        }
    }
}
