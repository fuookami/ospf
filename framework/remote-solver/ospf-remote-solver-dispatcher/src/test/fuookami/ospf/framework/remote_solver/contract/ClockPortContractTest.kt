package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.protocol.port.ClockPort
import kotlin.test.Test
import kotlin.test.assertTrue

abstract class ClockPortContractTest {
    protected abstract fun createFixture(): TestFixture<ClockPort>

    @Test
    fun nowEpochMsShouldBePositive() {
        val fixture = createFixture()
        try {
            val now = fixture.subject.nowEpochMs()
            assertTrue(now > 0L)
        } finally {
            fixture.close()
        }
    }

    @Test
    fun nowEpochMsShouldBeNonDecreasing() {
        val fixture = createFixture()
        try {
            val first = fixture.subject.nowEpochMs()
            val second = fixture.subject.nowEpochMs()
            assertTrue(second >= first)
        } finally {
            fixture.close()
        }
    }
}
