package fuookami.ospf.framework.remote_solver.protocol.domain

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.time.Duration
import fuookami.ospf.kotlin.math.algebra.number.Flt64

/**
 * 远程结果协议校验回归测试。 / Remote result protocol validation regression tests.
 */
class RemoteResultValidatorTest {
    /**
     * 验证严格 v2 结果的正交状态和归属字段。 /
     * Verify orthogonal state and ownership fields for a strict v2 result.
     */
    @Test
    fun strictV2SliceResultAcceptsMatchingOwnership() {
        val result = SliceResult(
            sliceId = SliceId.of("slice-1"),
            completed = false,
            feasible = true,
            objectiveValue = Flt64(3.0),
            gap = Flt64(0.5),
            elapsed = Duration.ZERO,
            schemaVersion = "2.0",
            problemStatus = RemoteProblemStatus.FEASIBLE,
            solutionPresence = RemoteSolutionPresence.INCUMBENT,
            proofStatus = RemoteProofStatus.CLAIMED,
            fingerprints = mapOf("model" to "m", "configuration" to "c", "solver" to "s"),
            fingerprintSchemas = mapOf("model" to "1.0", "configuration" to "1.0", "solver" to "2.0"),
            runId = "task-1",
            attemptId = "slice-1"
        )

        assertNull(
            RemoteResultValidator.validateSliceResult(
                result = result,
                expectedTaskId = TaskId.of("task-1"),
                expectedSliceId = SliceId.of("slice-1")
            )
        )
    }

    /**
     * 验证严格 v2 结果不能跨任务或跨未来 schema 混用。 /
     * Verify strict v2 results cannot cross task ownership or future schemas.
     */
    @Test
    fun strictV2RejectsOwnershipMismatchAndFutureSchema() {
        val base = SolveResult(
            feasible = true,
            optimal = false,
            objectiveValue = Flt64.one,
            gap = Flt64.zero,
            elapsed = Duration.ZERO,
            schemaVersion = "2.0",
            problemStatus = RemoteProblemStatus.FEASIBLE,
            solutionPresence = RemoteSolutionPresence.INCUMBENT,
            proofStatus = RemoteProofStatus.CLAIMED,
            fingerprints = mapOf("model" to "m"),
            fingerprintSchemas = mapOf("model" to "1.0"),
            runId = "task-1",
            attemptId = "slice-1"
        )

        val ownershipError = RemoteResultValidator.validateSolveResult(
            result = base,
            expectedTaskId = TaskId.of("task-2"),
            expectedAttemptId = SliceId.of("slice-1")
        )
        assertEquals(true, ownershipError?.contains("runId") == true)
        assertEquals(
            true,
            RemoteResultValidator.validateSolveResult(base.copy(schemaVersion = "3.0")).orEmpty()
                .contains("schema")
        )
        assertEquals(
            true,
            RemoteResultValidator.validateSolveResult(
                base.copy(fingerprintSchemas = emptyMap())
            ).orEmpty().contains("指纹")
        )
    }

    /**
     * 验证无 incumbent 结果不得携带目标值或错误解存在性。 /
     * Verify results without an incumbent cannot carry an objective or an invalid presence.
     */
    @Test
    fun validatorRejectsObjectiveWithoutIncumbent() {
        val invalid = SolveResult(
            feasible = false,
            optimal = false,
            objectiveValue = Flt64.one,
            gap = null,
            elapsed = Duration.ZERO,
            problemStatus = RemoteProblemStatus.UNKNOWN,
            solutionPresence = RemoteSolutionPresence.NONE
        )

        assertEquals(
            true,
            RemoteResultValidator.validateSolveResult(invalid)?.contains("objective") == true
        )
    }

    /**
     * 验证旧 v1 结果仍可读取，但不获得严格 v2 的归属门禁。 /
     * Verify legacy v1 results remain readable without strict v2 ownership gates.
     */
    @Test
    fun validatorKeepsLegacyV1Readable() {
        val legacy = SolveResult(
            feasible = true,
            optimal = false,
            objectiveValue = Flt64.one,
            gap = null,
            elapsed = Duration.ZERO,
            schemaVersion = "1.0",
            problemStatus = RemoteProblemStatus.FEASIBLE,
            solutionPresence = RemoteSolutionPresence.INCUMBENT
        )

        assertNull(
            RemoteResultValidator.validateSolveResult(
                result = legacy,
                expectedTaskId = TaskId.of("other-task"),
                expectedAttemptId = SliceId.of("other-slice")
            )
        )
    }
}
