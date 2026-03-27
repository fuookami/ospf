package fuookami.ospf.framework.remote_solver.adapter.mock

import fuookami.ospf.framework.remote_solver.protocol.domain.*
import fuookami.ospf.framework.remote_solver.protocol.port.ClockPort
import fuookami.ospf.framework.remote_solver.protocol.port.IdGeneratorPort
import fuookami.ospf.framework.remote_solver.protocol.port.ObjectStoragePort
import fuookami.ospf.framework.remote_solver.protocol.port.SolverExecutionPort
import kotlin.math.roundToLong
import kotlin.time.Duration

/**
 * Mock implementation of SolverExecutionPort for testing.
 *
 * This implementation simulates solver execution without calling actual OSPF solvers.
 * It tracks progress from 0.0 to 1.0 over a configurable simulated runtime.
 *
 * **IMPORTANT**: This is a mock for testing only. Do not use in production.
 * For production, use OspfSolverExecutionPort with OspfInProcessBridge.
 *
 * @param clock Clock port for timing
 * @param idGenerator ID generator for handles
 * @param objectStoragePort Object storage for results
 * @param simulatedTotalRuntimeMs Simulated total runtime in milliseconds (used for elapsedMs calculation)
 * @param progressPerSlice Progress to make per slice (0.0 to 1.0). Default 1.0 means complete in one slice.
 * @param simulatedObjectiveValue Objective value to report
 * @param alwaysFeasible Whether to always report feasible solutions
 * @param alwaysOptimal Whether to always report optimal solutions
 */
class MockSolverExecutionPort(
    private val clock: ClockPort,
    private val idGenerator: IdGeneratorPort,
    private val objectStoragePort: ObjectStoragePort,
    private val simulatedTotalRuntimeMs: Long = 12000L,
    private val progressPerSlice: Double = 1.0,
    private val simulatedObjectiveValue: Double = 0.0,
    private val alwaysFeasible: Boolean = true,
    private val alwaysOptimal: Boolean = true
) : SolverExecutionPort {

    private data class ExecutionContext(
        val tenantId: String,
        val taskId: String,
        val sliceId: String,
        val nodeId: String,
        val payload: SolvePayload,
        var progress: Double,
        var elapsedMs: Long,
        var checkpoint: ObjectRef? = null
    )

    private val contextsByHandle = linkedMapOf<String, ExecutionContext>()
    private val progressByCheckpoint = linkedMapOf<String, Double>()

    /**
     * Controls whether the mock should report feasibility.
     * Can be changed during tests to simulate different outcomes.
     */
    var reportFeasible: Boolean = alwaysFeasible

    /**
     * Controls whether the mock should report optimality.
     * Can be changed during tests to simulate different outcomes.
     */
    var reportOptimal: Boolean = alwaysOptimal

    /**
     * Custom objective value to report.
     * Can be changed during tests.
     */
    var customObjectiveValue: Double? = null

    override suspend fun start(
        payload: SolvePayload,
        taskId: TaskId,
        sliceId: SliceId,
        nodeId: NodeId,
        tenantId: TenantId
    ): ExecutionHandle {
        val handle = newHandle(taskId.value, sliceId.value, nodeId.value)
        contextsByHandle[handle.handleId.value] = ExecutionContext(
            tenantId = tenantId.value,
            taskId = taskId.value,
            sliceId = sliceId.value,
            nodeId = nodeId.value,
            payload = payload,
            progress = 0.0,
            elapsedMs = 0L
        )
        return handle
    }

    override suspend fun resume(
        payload: SolvePayload,
        checkpoint: ObjectRef,
        taskId: TaskId,
        sliceId: SliceId,
        nodeId: NodeId,
        tenantId: TenantId
    ): ExecutionHandle {
        val restoredProgress = restoreProgress(checkpoint)
        val handle = newHandle(taskId.value, sliceId.value, nodeId.value)
        contextsByHandle[handle.handleId.value] = ExecutionContext(
            tenantId = tenantId.value,
            taskId = taskId.value,
            sliceId = sliceId.value,
            nodeId = nodeId.value,
            payload = payload,
            progress = restoredProgress,
            elapsedMs = (simulatedTotalRuntimeMs * restoredProgress).roundToLong(),
            checkpoint = checkpoint
        )
        return handle
    }

    override suspend fun awaitSliceEnd(handle: ExecutionHandle, quantum: Duration): SliceResult {
        val context = contextsByHandle[handle.handleId.value]
            ?: return SliceResult(
                sliceId = handle.sliceId.value,
                completed = false,
                feasible = false,
                objectiveValue = null,
                gap = null,
                elapsedMs = 0L,
                message = "Execution handle not found"
            )

        val safeQuantumMs = quantum.inWholeMilliseconds.coerceAtLeast(1L)
        context.elapsedMs += safeQuantumMs
        // Infer progress based on model path - "simple" or "burst" paths complete in one slice
        val modelPath = context.payload.modelData.ref?.path?.value ?: ""
        val effectiveProgressPerSlice = inferProgressPerSlice(modelPath, progressPerSlice)
        val progressIncrement = if (effectiveProgressPerSlice >= 1.0) {
            1.0  // Complete in one slice
        } else {
            effectiveProgressPerSlice.coerceIn(0.0, 1.0)
        }
        context.progress = (context.progress + progressIncrement).coerceAtMost(1.0)

        val completed = context.progress >= 1.0
        val objectiveValue = customObjectiveValue
            ?: (simulatedObjectiveValue * (1.0 - context.progress)).coerceAtLeast(0.0)
        val gap = if (completed && reportOptimal) 0.0 else (1.0 - context.progress)

        return SliceResult(
            sliceId = handle.sliceId.value,
            completed = completed,
            feasible = reportFeasible,
            objectiveValue = objectiveValue,
            gap = gap,
            elapsedMs = safeQuantumMs,
            message = if (completed) "Mock completed" else "Mock suspended for next slice"
        )
    }

    /**
     * Infers progress per slice based on the model path.
     * Paths containing "simple" or "burst" are treated as simple tasks (complete in one slice).
     * Other paths use the configured progressPerSlice.
     */
    private fun inferProgressPerSlice(modelPath: String, defaultProgressPerSlice: Double): Double {
        val pathLower = modelPath.lowercase()
        return when {
            pathLower.contains("simple") || pathLower.contains("burst") -> 1.0
            else -> defaultProgressPerSlice
        }
    }

    override suspend fun exportCheckpoint(handle: ExecutionHandle): ObjectRef? {
        val context = contextsByHandle[handle.handleId.value] ?: return null
        val checkpointRef = objectStoragePort.put(
            path = "${context.tenantId}/checkpoint/${context.taskId}/${context.sliceId}",
            bytes = context.progress.toString().toByteArray(),
            metadata = mapOf(
                "taskId" to context.taskId,
                "sliceId" to context.sliceId,
                "tenantId" to context.tenantId,
                "progress" to context.progress.toString()
            )
        )
        context.checkpoint = checkpointRef
        progressByCheckpoint[checkpointKey(checkpointRef)] = context.progress
        return checkpointRef
    }

    override suspend fun fetchFinalResult(handle: ExecutionHandle): SolveResult? {
        val context = contextsByHandle[handle.handleId.value] ?: return null
        if (context.progress < 1.0) {
            return null
        }
        return SolveResult(
            feasible = reportFeasible,
            optimal = reportFeasible && reportOptimal,
            objectiveValue = customObjectiveValue ?: simulatedObjectiveValue,
            gap = if (reportOptimal) 0.0 else null,
            elapsedMs = context.elapsedMs,
            checkpointRef = context.checkpoint,
            message = "Mock solver result"
        )
    }

    override suspend fun stop(handle: ExecutionHandle): Boolean =
        contextsByHandle.remove(handle.handleId.value) != null

    /**
     * Clear all execution contexts.
     * Useful for test cleanup.
     */
    fun clear() {
        contextsByHandle.clear()
        progressByCheckpoint.clear()
    }

    /**
     * Get current progress for a handle.
     * Useful for testing assertions.
     */
    fun getProgress(handle: ExecutionHandle): Double? =
        contextsByHandle[handle.handleId.value]?.progress

    private suspend fun restoreProgress(checkpoint: ObjectRef): Double {
        progressByCheckpoint[checkpointKey(checkpoint)]?.let { return it }
        val bytes = objectStoragePort.get(checkpoint) ?: return 0.0
        return bytes.decodeToString().toDoubleOrNull()?.coerceIn(0.0, 1.0) ?: 0.0
    }

    private fun newHandle(taskId: String, sliceId: String, nodeId: String): ExecutionHandle =
        ExecutionHandle(
            handleId = idGenerator.newId("handle"),
            taskId = taskId,
            sliceId = sliceId,
            nodeId = nodeId,
            startedAtEpochMs = clock.nowEpochMs()
        )

    private fun checkpointKey(ref: ObjectRef): String = "${ref.path.value}@${ref.version?.value ?: "latest"}"
}

/**
 * Creates a MockSolverExecutionPort with default settings.
 */
fun mockSolverExecutionPort(
    clock: ClockPort,
    idGenerator: IdGeneratorPort,
    objectStoragePort: ObjectStoragePort,
    simulatedRuntimeMs: Long = 12000L
): MockSolverExecutionPort = MockSolverExecutionPort(
    clock = clock,
    idGenerator = idGenerator,
    objectStoragePort = objectStoragePort,
    simulatedTotalRuntimeMs = simulatedRuntimeMs
)
