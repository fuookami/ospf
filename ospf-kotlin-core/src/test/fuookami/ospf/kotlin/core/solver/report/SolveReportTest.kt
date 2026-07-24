package fuookami.ospf.kotlin.core.solver.report

import kotlin.test.*
import fuookami.ospf.kotlin.utils.functional.ok

class SolveReportTest {
    @Test
    fun shouldKeepProblemTerminationAndSolutionPresenceOrthogonal() {
        val report = SolveReport<Double>(
            problemStatus = ProblemStatus.Feasible,
            terminationReason = TerminationReason.TimeLimit,
            solutionPresence = SolutionPresence.Incumbent,
            solution = SolveSolution(
                values = listOf(1.0),
                objective = 1.0
            )
        )

        assertEquals(ProblemStatus.Feasible, report.problemStatus)
        assertEquals(TerminationReason.TimeLimit, report.terminationReason)
        assertEquals(SolutionPresence.Incumbent, report.solutionPresence)
    }

    @Test
    fun shouldCancelOnlyOnceAndKeepFirstCancellationFact() {
        var interruptionCount = 0
        val handle = SolveHandle.create {
            interruptionCount += 1
            ok
        }

        handle.cancel(CancellationSource.Remote, "用户取消")
        handle.cancel(CancellationSource.Timeout, "超时")

        assertEquals(1, interruptionCount)
        assertTrue(handle.token.isCancellationRequested)
        assertEquals(CancellationSource.Remote, handle.token.record?.source)
        assertEquals("用户取消", handle.token.record?.reason)
    }

    @Test
    fun shouldProduceStableCryptographicFingerprints() {
        val first = SolveFingerprinting.configuration(mapOf("threads" to "4", "seed" to "7"))
        val reordered = SolveFingerprinting.configuration(mapOf("seed" to "7", "threads" to "4"))
        val changed = SolveFingerprinting.configuration(mapOf("seed" to "8", "threads" to "4"))

        assertEquals(first, reordered)
        assertNotEquals(first, changed)
        assertEquals("SHA-256", first.algorithm)
        assertEquals(64, first.value.length)
    }
}
