/**
 * 远程求解器客户端 / Remote solver client
*/
package fuookami.ospf.kotlin.framework.solver.remote.client

import kotlin.time.Duration
import fuookami.ospf.kotlin.utils.error.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.math.algebra.number.*
import fuookami.ospf.kotlin.framework.solver.remote.domain.*
import fuookami.ospf.kotlin.framework.solver.remote.port.SolverExecutionPort

/**
 * 远程求解器客户端。 / Remote solver client.
 *
 * @property executionPort 求解执行端口 / Solve execution port
*/
class RemoteSolverClient(
    private val executionPort: SolverExecutionPort
) {

    /**
     * 执行远程求解。 / Execute remote solve.
     *
     * @param payload 求解载荷 / Solve payload
     * @param taskId 任务 ID / Task ID
     * @param sliceId 切片 ID / Slice ID
     * @param nodeId 节点 ID / Node ID
     * @param tenantId 租户 ID / Tenant ID
     * @param quantum 时间片 / Quantum
     * @param maxRounds 最大轮数 / Maximum rounds
     * @param exportCheckpointEachRound 是否每轮导出检查点 / Whether export checkpoint each round
     * @return 求解结果 / Solve result
    */
    suspend fun solve(
        payload: SolvePayload,
        taskId: TaskId,
        sliceId: SliceId,
        nodeId: NodeId,
        tenantId: TenantId,
        quantum: Duration,
        maxRounds: UInt64 = UInt64(64),
        exportCheckpointEachRound: Boolean = true
    ): Ret<SolveResult> {
        if (quantum <= Duration.ZERO) {
            return Failed(ErrorCode.IllegalArgument, "quantum must be positive.")
        }
        if (maxRounds <= UInt64.zero) {
            return Failed(ErrorCode.IllegalArgument, "maxRounds must be positive.")
        }

        val handle = when (val result = payload.snapshotRef?.let {
            executionPort.resume(
                payload = payload,
                checkpoint = it,
                taskId = taskId,
                sliceId = sliceId,
                nodeId = nodeId,
                tenantId = tenantId
            )
        } ?: executionPort.start(
            payload = payload,
            taskId = taskId,
            sliceId = sliceId,
            nodeId = nodeId,
            tenantId = tenantId
        )) {
            is Ok -> result.value
            is Failed -> return Failed(result.error)
            is Fatal -> return Fatal(result.errors)
        }

        var totalElapsed = Duration.ZERO
        var latestCheckpoint: ObjectRef? = payload.snapshotRef
        var rounds = UInt64.zero
        var finalResult: SolveResult? = null
        while (rounds < maxRounds) {
            rounds += UInt64.one
            val sliceResult = when (val result = executionPort.awaitSliceEnd(
                handle = handle,
                quantum = quantum
            )) {
                is Ok -> result.value
                is Failed -> return Failed(result.error)
                is Fatal -> return Fatal(result.errors)
            }
            totalElapsed += sliceResult.elapsed
            if (exportCheckpointEachRound) {
                latestCheckpoint = when (val result = executionPort.exportCheckpoint(handle)) {
                    is Ok -> result.value ?: latestCheckpoint
                    is Failed -> return Failed(result.error)
                    is Fatal -> return Fatal(result.errors)
                }
            }
            if (sliceResult.completed) {
                val fetchedResult = when (val result = executionPort.fetchFinalResult(handle)) {
                    is Ok -> result.value
                    is Failed -> return Failed(result.error)
                    is Fatal -> return Fatal(result.errors)
                }
                finalResult = fetchedResult ?: SolveResult(
                    feasible = sliceResult.feasible,
                    optimal = sliceResult.solutionPresence == RemoteSolutionPresence.OPTIMAL &&
                        sliceResult.proofStatus != RemoteProofStatus.NONE,
                    objectiveValue = sliceResult.objectiveValue,
                    objectiveValueInt64 = sliceResult.objectiveValueInt64,
                    gap = sliceResult.gap,
                    elapsed = totalElapsed,
                    checkpointRef = latestCheckpoint,
                    message = sliceResult.message,
                    schemaVersion = sliceResult.schemaVersion,
                    problemStatus = sliceResult.problemStatus,
                    terminationReason = sliceResult.terminationReason,
                    solutionPresence = sliceResult.solutionPresence,
                    proofStatus = sliceResult.proofStatus,
                    resultRef = sliceResult.resultRef,
                    provenance = sliceResult.provenance,
                    fingerprints = sliceResult.fingerprints,
                    fingerprintSchemas = sliceResult.fingerprintSchemas,
                    statistics = sliceResult.statistics,
                    diagnostics = sliceResult.diagnostics,
                    runId = sliceResult.runId,
                    attemptId = sliceResult.attemptId,
                    artifactDigest = sliceResult.artifactDigest
                )
                break
            }
        }
        executionPort.stop(handle)

        return finalResult?.let { Ok(it) } ?: Failed(
            ExErr(
                code = ErrorCode.ApplicationFailed,
                message = "Remote solve does not complete within maxRounds=$maxRounds (taskId=$taskId, sliceId=$sliceId).",
                value = RemoteSolverFailureDetail(
                    code = RemoteSolverErrorCode.REMOTE_SOLVE_NOT_COMPLETED_WITHIN_MAX_ROUNDS,
                    message = "Remote solve does not complete within maxRounds=$maxRounds.",
                    metadata = mapOf(
                        "taskId" to taskId.value,
                        "sliceId" to sliceId.value,
                        "maxRounds" to maxRounds.toString()
                    ),
                    taskId = taskId.value,
                    sliceId = sliceId.value
                )
            )
        )
    }
}
