package fuookami.ospf.framework.remote_solver.protocol.domain

/**
 * 远程结果协议边界校验器。 / Remote result protocol boundary validator.
 *
 * The validator checks only transport invariants shared by all solver backends. Mathematical
 * solution validation remains the responsibility of the backend adapter and client model view.
 * 校验器只检查所有后端共用的传输不变量；数学解复验仍由后端适配器和客户端模型视图负责。
 */
object RemoteResultValidator {
    private val supportedMajorVersions = setOf(1, 2)

    /**
     * 校验切片结果。 / Validate a slice result.
     *
     * @param result 待校验的切片结果 / Slice result to validate
     * @param expectedTaskId 当前任务 ID / Current task identifier
     * @param expectedSliceId 当前切片 ID / Current slice identifier
     * @return null 表示通过，否则返回结构化错误消息 / Null when valid, otherwise an error message
     */
    fun validateSliceResult(
        result: SliceResult,
        expectedTaskId: TaskId? = null,
        expectedSliceId: SliceId? = null
    ): String? {
        if (expectedSliceId != null && result.sliceId != expectedSliceId) {
            return "sliceId 与执行句柄不一致 / sliceId does not match the execution handle"
        }
        return validateCommon(
            feasible = result.feasible,
            optimal = result.solutionPresence == RemoteSolutionPresence.OPTIMAL,
            objectiveValue = result.objectiveValue,
            objectiveValueInt64 = result.objectiveValueInt64,
            problemStatus = result.problemStatus,
            solutionPresence = result.solutionPresence,
            proofStatus = result.proofStatus,
            schemaVersion = result.schemaVersion,
            resultRef = result.resultRef,
            artifactDigest = result.artifactDigest,
            fingerprints = result.fingerprints,
            fingerprintSchemas = result.fingerprintSchemas,
            runId = result.runId,
            attemptId = result.attemptId,
            expectedTaskId = expectedTaskId?.value,
            expectedAttemptId = expectedSliceId?.value
        )
    }

    /**
     * 校验最终结果。 / Validate a final result.
     *
     * @param result 待校验的最终结果 / Final result to validate
     * @param expectedTaskId 当前任务 ID / Current task identifier
     * @param expectedAttemptId 当前切片 ID / Current slice identifier
     * @return null 表示通过，否则返回结构化错误消息 / Null when valid, otherwise an error message
     */
    fun validateSolveResult(
        result: SolveResult,
        expectedTaskId: TaskId? = null,
        expectedAttemptId: SliceId? = null
    ): String? {
        return validateCommon(
            feasible = result.feasible,
            optimal = result.optimal,
            objectiveValue = result.objectiveValue,
            objectiveValueInt64 = result.objectiveValueInt64,
            problemStatus = result.problemStatus,
            solutionPresence = result.solutionPresence,
            proofStatus = result.proofStatus,
            schemaVersion = result.schemaVersion,
            resultRef = result.resultRef,
            artifactDigest = result.artifactDigest,
            fingerprints = result.fingerprints,
            fingerprintSchemas = result.fingerprintSchemas,
            runId = result.runId,
            attemptId = result.attemptId,
            expectedTaskId = expectedTaskId?.value,
            expectedAttemptId = expectedAttemptId?.value
        )
    }

    private fun validateCommon(
        feasible: Boolean,
        optimal: Boolean,
        objectiveValue: Any?,
        objectiveValueInt64: Long?,
        problemStatus: RemoteProblemStatus,
        solutionPresence: RemoteSolutionPresence,
        proofStatus: RemoteProofStatus,
        schemaVersion: String,
        resultRef: ObjectRef?,
        artifactDigest: String?,
        fingerprints: Map<String, String>,
        fingerprintSchemas: Map<String, String>,
        runId: String?,
        attemptId: String?,
        expectedTaskId: String?,
        expectedAttemptId: String?
    ): String? {
        val major = schemaVersion.substringBefore('.').toIntOrNull()
            ?: return "结果 schema 无效 / Invalid result schema"
        if (major !in supportedMajorVersions) {
            return "不支持的结果 schema 主版本：$schemaVersion / Unsupported result schema major: $schemaVersion"
        }
        val strict = major >= 2
        if (strict && schemaVersion != "2.0") {
            return "不支持的结果 schema：$schemaVersion / Unsupported result schema: $schemaVersion"
        }
        if ((feasible && problemStatus != RemoteProblemStatus.FEASIBLE) ||
            (!feasible && problemStatus == RemoteProblemStatus.FEASIBLE)
        ) {
            return "问题状态与可行性不一致 / Problem status disagrees with feasibility"
        }
        val expectedPresence = when {
            optimal -> RemoteSolutionPresence.OPTIMAL
            feasible -> RemoteSolutionPresence.INCUMBENT
            else -> RemoteSolutionPresence.NONE
        }
        if (solutionPresence != expectedPresence) {
            return "解存在性与状态不一致 / Solution presence disagrees with status"
        }
        if (optimal && strict && proofStatus == RemoteProofStatus.NONE) {
            return "严格最优结果缺少证明状态 / Strict optimal result is missing proof status"
        }
        if (!feasible && (objectiveValue != null || objectiveValueInt64 != null)) {
            return "无 incumbent 结果不得携带目标值 / A result without an incumbent must not carry an objective"
        }
        if (objectiveValue != null && objectiveValueInt64 != null) {
            return "结果不得同时携带浮点和 Int64 目标 / A result must not carry both floating and Int64 objectives"
        }
        if ((resultRef == null) != (artifactDigest == null)) {
            return "resultRef 与 artifactDigest 必须成对出现 / resultRef and artifactDigest must be provided together"
        }
        if (strict) {
            if (fingerprints.keys != fingerprintSchemas.keys ||
                fingerprints.values.any(String::isBlank) ||
                fingerprintSchemas.values.any(String::isBlank)
            ) {
                return "严格结果的指纹 schema 不完整 / Strict result fingerprint schemas are incomplete"
            }
            if (runId.isNullOrBlank() || attemptId.isNullOrBlank()) {
                return "严格结果缺少 run/attempt 标识 / Strict result is missing run/attempt identifiers"
            }
            if (expectedTaskId != null && runId != expectedTaskId) {
                return "结果 runId 不属于当前任务 / Result runId does not belong to the current task"
            }
            if (expectedAttemptId != null && attemptId != expectedAttemptId) {
                return "结果 attemptId 不属于当前切片 / Result attemptId does not belong to the current slice"
            }
        }
        return null
    }
}
