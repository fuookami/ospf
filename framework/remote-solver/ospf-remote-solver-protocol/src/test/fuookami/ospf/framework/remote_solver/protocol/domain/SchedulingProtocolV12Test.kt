@file:OptIn(kotlin.time.ExperimentalTime::class)

package fuookami.ospf.framework.remote_solver.protocol.domain

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlin.time.Duration.Companion.milliseconds
import kotlin.time.Instant
import kotlinx.serialization.json.Json
import fuookami.ospf.kotlin.math.algebra.number.Flt64

/** Focused V1.2 scheduling protocol round-trip and semantic tests. */
class SchedulingProtocolV12Test {
    private val json = Json {
        encodeDefaults = true
        ignoreUnknownKeys = false
    }

    @Test
    fun legacyPayloadDefaultsSchedulingRequestToNull() {
        val legacy = SolvePayload(
            modelData = ModelData.reference(ObjectRef.of("models/legacy"))
        )

        val decoded = json.decodeFromString(
            SolvePayload.serializer(),
            json.encodeToString(SolvePayload.serializer(), legacy)
        )

        assertNull(decoded.scheduling)
        assertEquals("models/legacy", decoded.modelData.ref?.path?.value)
    }

    @Test
    fun schedulingRequestRoundTripsAllV12TaskFields() {
        val request = SchedulingRequest(
            complexity = TaskComplexity.COMPLEX,
            timeSensitivity = TimeSensitivity.NON_REALTIME,
            priority = 7,
            deadline = Instant.fromEpochMilliseconds(1_700_000_000_000L),
            budgetScope = BudgetScopeId.of("tenant-a"),
            budgetLimit = Flt64(12.5),
            estimate = SchedulingEstimate(
                runtime = 12_000.milliseconds,
                checkpoint = 350.milliseconds,
                queueWait = 80.milliseconds,
                cost = Flt64(3.25)
            ),
            modelFingerprint = "sha256:model",
            modelFingerprintSchema = "sha256-v1",
            checkpointRef = ObjectRef.of("checkpoints/task-1", version = "v2"),
            incumbentRef = ObjectRef.of("incumbents/task-1"),
            qualityTarget = QualityTarget(
                maxGap = Flt64(0.01),
                objectiveLimit = Flt64(100.0),
                requireFeasible = true,
                requireOptimal = false,
                metadata = mapOf("metric" to "relative-gap")
            ),
            preemptionMode = PreemptionMode.CONTROLLED_RETURN,
            resumeMode = ResumeMode.WARM_START,
            metadata = mapOf("requiresCheckpoint" to "true")
        )
        val payload = SolvePayload(
            modelData = ModelData.reference(ObjectRef.of("models/task-1")),
            scheduling = request
        )

        val encoded = json.encodeToString(SolvePayload.serializer(), payload)
        val decoded = json.decodeFromString(SolvePayload.serializer(), encoded)
        val actual = checkNotNull(decoded.scheduling)

        assertEquals(request, actual)
        assertTrue(encoded.contains("\"deadlineEpochMs\":1700000000000"))
        assertTrue(encoded.contains("\"estimatedRuntimeMs\":12000"))
        assertTrue(encoded.contains("\"checkpointRef\""))
        assertTrue(encoded.contains("\"qualityTarget\""))
    }

    @Test
    fun sliceResponseRoundTripsOutcomeAndEffectiveScheduling() {
        val scheduling = SchedulingDecision(
            dispatchId = DispatchId.of("dispatch-1"),
            taskId = TaskId.of("task-1"),
            sliceId = SliceId.of("slice-1"),
            nodeId = NodeId.of("node-1"),
            priority = 7,
            deadline = Instant.fromEpochMilliseconds(1_700_000_000_000L),
            budgetScope = BudgetScopeId.of("tenant-a"),
            budgetLimit = Flt64(12.5),
            quantum = 2_000.milliseconds,
            queueWait = 125.milliseconds,
            estimate = SchedulingEstimate(checkpoint = 350.milliseconds),
            modelFingerprint = "sha256:model",
            checkpointRef = ObjectRef.of("checkpoints/task-1/slice-1"),
            incumbentRef = ObjectRef.of("incumbents/task-1"),
            preemptionMode = PreemptionMode.CONTROLLED_RETURN,
            resumeMode = ResumeMode.WARM_START,
            outcome = SliceOutcome.CHECKPOINTED,
            reason = "quantum-expired",
            metadata = mapOf("selection" to "weighted-round-robin")
        )
        val result = SliceResult(
            sliceId = SliceId.of("slice-1"),
            completed = false,
            feasible = true,
            objectiveValue = Flt64(42.0),
            gap = Flt64(0.2),
            elapsed = 1_900.milliseconds,
            checkpointRef = ObjectRef.of("checkpoints/task-1/slice-1"),
            incumbentRef = ObjectRef.of("incumbents/task-1"),
            modelFingerprint = "sha256:model",
            scheduling = scheduling,
            outcome = SliceOutcome.CHECKPOINTED
        )

        val decoded = json.decodeFromString(
            SliceResult.serializer(),
            json.encodeToString(SliceResult.serializer(), result)
        )

        assertEquals(SliceOutcome.CHECKPOINTED, decoded.outcome)
        assertEquals(PreemptionMode.CONTROLLED_RETURN, decoded.scheduling?.preemptionMode)
        assertEquals(ResumeMode.WARM_START, decoded.scheduling?.resumeMode)
        assertEquals(2_000L, decoded.scheduling?.quantum?.inWholeMilliseconds)
        assertEquals(result.checkpointRef, decoded.checkpointRef)
        assertEquals(result.incumbentRef, decoded.incumbentRef)
        assertEquals("sha256:model", decoded.modelFingerprint)
    }

    @Test
    fun futureSchedulingFieldsCanBeIgnoredByForwardCompatibleReader() {
        val encoded = """
            {
              "modelData": {"ref": {"path": "models/task-1"}},
              "scheduling": {
                "complexity": "SIMPLE",
                "futurePolicy": "fair-share"
              }
            }
        """.trimIndent()
        val decoded = Json { ignoreUnknownKeys = true }.decodeFromString(
            SolvePayload.serializer(),
            encoded
        )

        assertEquals(TaskComplexity.SIMPLE, decoded.scheduling?.complexity)
        assertNull(decoded.scheduling?.priority)
    }
}
