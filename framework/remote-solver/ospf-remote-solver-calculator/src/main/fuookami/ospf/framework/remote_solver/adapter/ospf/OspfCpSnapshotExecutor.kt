@file:OptIn(kotlin.time.ExperimentalTime::class)

/**
 * OSPF portable CP snapshot executor。
 * OSPF portable CP snapshot executor.
 */
package fuookami.ospf.framework.remote_solver.adapter.ospf

import kotlin.time.Duration
import kotlin.time.DurationUnit
import kotlin.time.Duration.Companion.milliseconds
import kotlin.time.toDuration
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import fuookami.ospf.kotlin.utils.functional.Failed
import fuookami.ospf.kotlin.utils.functional.Fatal
import fuookami.ospf.kotlin.utils.functional.Ok
import fuookami.ospf.kotlin.utils.functional.Ret
import fuookami.ospf.kotlin.utils.functional.ok
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.algebra.number.Int64
import fuookami.ospf.kotlin.math.algebra.number.UInt64
import fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingModel
import fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingModelSnapshot
import fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingSnapshotCodec
import fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingCheckpointCodec
import fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingSolveOptions
import fuookami.ospf.kotlin.core.solver.report.CancellationToken
import fuookami.ospf.kotlin.core.solver.output.ConstraintProgrammingFeasibleOutput
import fuookami.ospf.kotlin.core.solver.output.ConstraintProgrammingInfeasibleOutput
import fuookami.ospf.kotlin.core.solver.output.ConstraintProgrammingSolution
import fuookami.ospf.kotlin.core.solver.output.ConstraintProgrammingSolverOutput
import fuookami.ospf.kotlin.core.solver.output.ConstraintProgrammingUnknownOutput
import fuookami.ospf.kotlin.core.solver.report.SolutionPresence
import fuookami.ospf.kotlin.core.solver.report.TerminationReason
import fuookami.ospf.kotlin.core.solver.report.InfeasibilityMember
import fuookami.ospf.kotlin.core.solver.report.SolveReport
import fuookami.ospf.kotlin.core.solver.scip.ScipConstraintProgrammingSolver
import fuookami.ospf.kotlin.core.solver.scip.scipRuntimeFingerprint
import fuookami.ospf.kotlin.core.solver.scip.scipRuntimeFingerprintLegacyV1Candidates
import fuookami.ospf.kotlin.core.solver.report.VariableId
import fuookami.ospf.kotlin.core.variable.AbstractVariableItem
import fuookami.ospf.kotlin.core.variable.BinVar
import fuookami.ospf.kotlin.core.variable.IntVar
import fuookami.ospf.framework.remote_solver.protocol.domain.PortableCheckpointCodec
import fuookami.ospf.framework.remote_solver.protocol.domain.PortableCheckpointEnvelope
import fuookami.ospf.framework.remote_solver.protocol.domain.PortableConstraintProgrammingConflict
import fuookami.ospf.framework.remote_solver.protocol.domain.PortableConstraintProgrammingIncumbent
import fuookami.ospf.framework.remote_solver.protocol.domain.PortableConstraintProgrammingIntervalValue
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteProblemStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteProofStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteSolutionPresence
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteTerminationReason
import fuookami.ospf.framework.remote_solver.protocol.domain.SerializedIntervalValue
import fuookami.ospf.framework.remote_solver.protocol.domain.SerializedSolution
import fuookami.ospf.framework.remote_solver.protocol.domain.SolvePayload
import fuookami.ospf.framework.remote_solver.protocol.domain.SolveResult
import fuookami.ospf.framework.remote_solver.protocol.domain.SolverConfig
import fuookami.ospf.framework.remote_solver.protocol.port.ObjectStoragePort

private const val DEFAULT_SCIP_THREADS = 8
private const val CP_RESULT_SCHEMA_VERSION = "2.0"

/**
 * Exact CP objective fields emitted at every protocol boundary.
 * 每个协议边界写出的 CP 精确目标字段。
 */
internal data class CpObjectiveFields(
    val objectiveValue: Flt64? = null,
    val objectiveValueInt64: Long? = null
)

/**
 * Map an exact CP integer objective without passing through floating point.
 * 将 CP 精确整数目标映射为协议字段，避免经过浮点数。
 *
 * @param value 精确整数目标 / Exact integer objective
 * @return CP 目标字段 / CP objective fields
 */
internal fun exactCpObjectiveFields(value: Int64?): CpObjectiveFields {
    return CpObjectiveFields(objectiveValueInt64 = value?.toLong())
}

/**
 * 执行 portable CP snapshot，并生成稳定结果 artifact。
 * Executes a portable CP snapshot and writes a stable result artifact.
 *
 * @property objectStoragePort 结果 artifact 存储端口 / Result artifact storage port
 */
class OspfCpSnapshotExecutor(
    private val objectStoragePort: ObjectStoragePort
) {
    private val json = Json {
        encodeDefaults = true
        ignoreUnknownKeys = false
    }

    /**
     * 执行 CP 载荷。
     * Executes a CP payload.
     *
     * @param payload CP 求解载荷 / CP solve payload
     * @param tenantId 租户 ID / Tenant ID
     * @param taskId 任务 ID / Task ID
     * @param sliceId 切片 ID / Slice ID
     * @param quantumMs 本次切片时间上限（毫秒）/ Slice time limit in milliseconds
     * @param cancellationToken 可选取消令牌 / Optional cancellation token
     * @param checkpoint 可选 portable checkpoint / Optional portable checkpoint
     * @return 求解结果 / Solve result
     */
    suspend fun execute(
        payload: SolvePayload,
        tenantId: String,
        taskId: String,
        sliceId: String,
        quantumMs: Long? = null,
        cancellationToken: CancellationToken? = null,
        checkpoint: PortableCheckpointEnvelope? = null
    ): SolveResult {
        val started = System.nanoTime()
        return try {
            validateConfiguration(payload)?.let { return failureResult(it, taskId, sliceId) }
            val encoded = loadSnapshot(payload)
                ?: return failureResult("CP snapshot artifact is missing", taskId, sliceId)
            val decoded = decodeModel(encoded)
            val restored = when (val validation = validateCheckpoint(
                payload = payload,
                decoded = decoded,
                encodedSnapshot = encoded,
                rawCheckpoint = checkpoint,
                taskId = taskId,
                sliceId = sliceId
            )) {
                is Ok -> validation.value
                is Failed -> return failureResult(validation.error.message, taskId, sliceId)
                is Fatal -> return failureResult(validation.errors.joinToString { it.message }, taskId, sliceId)
            }
            val output = solve(decoded.model, payload, quantumMs, cancellationToken, restored)
            val elapsed = (System.nanoTime() - started).toDuration(DurationUnit.NANOSECONDS)
            writeResultArtifact(
                tenantId = tenantId,
                taskId = taskId,
                sliceId = sliceId,
                execution = toExecution(
                    output = output,
                    decoded = decoded,
                    elapsed = elapsed,
                    encoded = encoded,
                    runId = taskId,
                    attemptId = sliceId
                )
            )
        } catch (error: Throwable) {
            failureResult(
                "CP snapshot execution failed: ${error.message ?: error::class.simpleName}",
                taskId,
                sliceId
            )
        }
    }

    /**
     * 导出 portable checkpoint v2；不保存 SCIP 原生搜索树或句柄。
     * Export a portable checkpoint v2 without persisting SCIP native search state or handles.
     *
     * @param payload 原始 CP 求解载荷 / Original CP solve payload
     * @param result 已完成的求解结果 / Completed solve result
     * @param tenantId 租户 ID / Tenant identifier
     * @param taskId 任务 ID / Task identifier
     * @param sliceId 切片 ID / Slice identifier
     * @param parentCheckpointId 父 checkpoint 标识 / Parent checkpoint identifier
     * @return checkpoint 对象引用，无法生成可信 artifact 时返回 null / Checkpoint reference, or null when a trusted artifact cannot be produced
     */
    suspend fun exportCheckpoint(
        payload: SolvePayload,
        result: SolveResult,
        tenantId: String,
        taskId: String,
        sliceId: String,
        parentCheckpointId: String? = null
    ): fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef? {
        return try {
            val snapshotJson = loadSnapshot(payload) ?: return null
            val snapshotRoot = json.parseToJsonElement(snapshotJson).jsonObject
            if (snapshotRoot["variables"] == null || snapshotRoot["constraints"] == null) {
                return null
            }
            val decoded = runCatching { decodeModel(snapshotJson) }.getOrNull() ?: return null
            val effectiveConfig = semanticConfiguration(payload)
            val configurationFingerprint = fingerprint(configurationCanonical(effectiveConfig))
            val modelFingerprint = fingerprint(snapshotJson)
            if (result.schemaVersion != CP_RESULT_SCHEMA_VERSION ||
                result.runId != taskId ||
                result.attemptId != sliceId ||
                result.fingerprints["model"] != modelFingerprint ||
                result.fingerprints["configuration"] != configurationFingerprint
            ) {
                return null
            }
            if (result.fingerprints["solver"].isNullOrBlank()) {
                return null
            }
            if (result.resultRef == null) {
                return null
            }
            val solution = loadVerifiedArtifact(
                result = result,
                tenantId = tenantId,
                taskId = taskId,
                sliceId = sliceId,
                modelFingerprint = modelFingerprint,
                configurationFingerprint = configurationFingerprint
            ) ?: return null
            if (!validateExternalSolution(snapshotJson, solution)) {
                return null
            }
            if (!solution.feasible &&
                (solution.objectiveValue != null || solution.objectiveValueInt64 != null)
            ) {
                return null
            }
            val checkpointId = "$sliceId-${System.currentTimeMillis()}"
            val incumbent = solution?.takeIf { it.feasible }?.let {
                val objective = it.objectiveValueInt64?.toString()
                    ?: canonicalObjective(it.objectiveValue)
                    ?: recomputeObjective(decoded.snapshot, it)
                if (decoded.snapshot.objectives.isNotEmpty() && objective == null) {
                    return null
                }
                PortableConstraintProgrammingIncumbent(
                    valuesById = it.variableValuesById,
                    intervalsById = it.intervalValues.mapValues { (_, value) ->
                        PortableConstraintProgrammingIntervalValue(
                            start = value.start,
                            size = value.size,
                            end = value.end,
                            present = value.present
                        )
                    },
                    objective = objective
                )
            }
            val assumptions = decodeDiagnosticList(result.diagnostics["infeasibility.assumptionIds"]) ?: return null
            val conflictMembers = decodeDiagnosticList(result.diagnostics["infeasibility.members"]) ?: return null
            val conflicts = if (conflictMembers.isEmpty()) {
                emptyList()
            } else {
                listOf(
                    PortableConstraintProgrammingConflict(
                        validity = result.diagnostics["infeasibility.validity"] ?: "Unknown",
                        minimality = result.diagnostics["infeasibility.minimality"] ?: "NotChecked",
                        memberIds = conflictMembers,
                        assumptionIds = assumptions,
                        provenance = result.diagnostics
                            .filterKeys { it.startsWith("infeasibility.") }
                            .filterValues { it.isNotBlank() }
                    )
                )
            }
            val envelope = PortableCheckpointEnvelope(
                checkpointId = checkpointId,
                identitySchemaVersion = snapshotRoot["identitySchemaVersion"]?.jsonPrimitive?.content ?: "1.0",
                identityNamespace = snapshotRoot["identityNamespace"]?.jsonPrimitive?.content ?: "model-local",
                modelName = snapshotRoot["name"]?.jsonPrimitive?.content ?: "remote-cp",
                modelFingerprint = fingerprint(snapshotJson),
                configurationFingerprint = configurationFingerprint,
                solverFingerprint = result.fingerprints["solver"] ?: fingerprint("scip-cp"),
                runId = taskId,
                attemptId = sliceId,
                parentCheckpointId = parentCheckpointId,
                createdAtEpochMs = System.currentTimeMillis(),
                snapshotJson = snapshotJson,
                incumbent = incumbent,
                bestBound = result.statistics["bestBound"],
                gap = result.gap?.toString() ?: result.statistics["gap"],
                assumptions = assumptions,
                conflicts = conflicts
            )
            val encoded = PortableCheckpointCodec.encode(envelope)
            val normalizedEnvelope = PortableCheckpointCodec.decodeOrNull(encoded) ?: return null
            when (val validation = validateCheckpoint(
                payload = payload,
                decoded = decoded,
                encodedSnapshot = snapshotJson,
                rawCheckpoint = normalizedEnvelope,
                configurationFingerprintOverride = configurationFingerprint,
                taskId = taskId,
                sliceId = sliceId
            )) {
                is Ok -> {}
                is Failed -> return null
                is Fatal -> return null
            }
            objectStoragePort.put(
                path = "$tenantId/checkpoint/$taskId/$checkpointId",
                bytes = encoded.encodeToByteArray(),
                metadata = mapOf(
                    "tenantId" to tenantId,
                    "taskId" to taskId,
                    "sliceId" to sliceId,
                    "contentType" to "application/json",
                    "schemaVersion" to normalizedEnvelope.schemaVersion,
                    "sourceFormat" to normalizedEnvelope.sourceFormat,
                    "modelFingerprint" to normalizedEnvelope.modelFingerprint,
                    "configurationFingerprint" to (normalizedEnvelope.configurationFingerprint ?: ""),
                    "solverFingerprint" to (normalizedEnvelope.solverFingerprint ?: ""),
                    "runId" to (normalizedEnvelope.runId ?: ""),
                    "attemptId" to (normalizedEnvelope.attemptId ?: ""),
                    "integritySha256" to normalizedEnvelope.integritySha256
                )
            )
        } catch (_: Throwable) {
            null
        }
    }

    private suspend fun loadSnapshot(payload: SolvePayload): String? {
        val inline = payload.modelData.rawBytes
        val encoded = if (inline != null) {
            inline.decodeToString()
        } else {
            val ref = payload.modelData.ref ?: return null
            objectStoragePort.get(ref)?.decodeToString() ?: return null
        }
        return canonicalSnapshotJson(encoded)
    }

    private suspend fun loadVerifiedArtifact(
        result: SolveResult,
        tenantId: String,
        taskId: String,
        sliceId: String,
        modelFingerprint: String,
        configurationFingerprint: String
    ): SerializedSolution? {
        val ref = result.resultRef ?: return null
        val expectedPath = "$tenantId/result/$taskId/$sliceId"
        if (ref.path.value != expectedPath || result.artifactDigest == null) {
            return null
        }
        val bytes = objectStoragePort.get(ref) ?: return null
        val solution = runCatching {
            val encoded = bytes.decodeToString()
            val root = json.parseToJsonElement(encoded).jsonObject
            val decoded = json.decodeFromString(SerializedSolution.serializer(), encoded)
            val rawSchemaVersion = root["schemaVersion"]?.jsonPrimitive?.content
            val rawMajorVersion = rawSchemaVersion?.substringBefore('.')?.toIntOrNull()
            val resultMajorVersion = result.schemaVersion.substringBefore('.').toIntOrNull()
            val hasNonEmptyFingerprintSchemas = root["fingerprintSchemas"]
                ?.jsonObject
                ?.isNotEmpty() == true
            // The raw result is authoritative for the protocol strictness.  An artifact must not
            // downgrade itself to the legacy decoder by omitting schemaVersion or audit fields.
            // raw result 的协议版本决定严格程度；artifact 不能通过省略 schemaVersion 或审计字段自行降级。
            val strict = resultMajorVersion?.let { it >= 2 } == true ||
                rawMajorVersion?.let { it >= 2 } == true ||
                hasNonEmptyFingerprintSchemas
            if (strict && rawMajorVersion != null && rawMajorVersion < 2) {
                return@runCatching null
            }
            if (strict) {
                val requiredFields = setOf(
                    "schemaVersion",
                    "feasible",
                    "optimal",
                    "objectiveValue",
                    "objectiveValueInt64",
                    "elapsedMs",
                    "message",
                    "variableValuesById",
                    "intervalValues",
                    "problemStatus",
                    "solutionPresence",
                    "proofStatus",
                    "terminationReason",
                    "gap",
                    "runId",
                    "attemptId",
                    "artifactDigest",
                    "provenance",
                    "fingerprints",
                    "fingerprintSchemas",
                    "statistics",
                    "diagnostics"
                )
                if (requiredFields.any { it !in root.keys }) {
                    return@runCatching null
                }
                val requiredNonNull = setOf(
                    "schemaVersion",
                    "problemStatus",
                    "solutionPresence",
                    "proofStatus",
                    "terminationReason",
                    "runId",
                    "attemptId",
                    "artifactDigest"
                )
                if (requiredNonNull.any { key ->
                        root[key]?.toString() == "null" ||
                            (root[key]?.jsonPrimitive?.isString == true && root[key]?.jsonPrimitive?.content.isNullOrBlank())
                    }
                ) {
                    return@runCatching null
                }
            }
            decoded
        }.getOrNull() ?: return null
        val unsigned = solution.copy(artifactDigest = null)
        val unsignedBytes = json.encodeToString(
            SerializedSolution.serializer(),
            unsigned
        ).encodeToByteArray()
        val computedDigest = fingerprint(unsignedBytes.decodeToString())
        if (solution.artifactDigest != computedDigest || result.artifactDigest != computedDigest) {
            return null
        }
        if (result.runId != taskId || result.attemptId != sliceId ||
            solution.runId != taskId || solution.attemptId != sliceId
        ) {
            return null
        }
        if (solution.feasible != result.feasible ||
            solution.optimal != result.optimal ||
            solution.objectiveValue != result.objectiveValue ||
            solution.objectiveValueInt64 != result.objectiveValueInt64 ||
            solution.gap != result.gap ||
            solution.problemStatus != result.problemStatus ||
            solution.solutionPresence != result.solutionPresence ||
            solution.proofStatus != result.proofStatus ||
            solution.terminationReason != result.terminationReason ||
            solution.schemaVersion != result.schemaVersion ||
            solution.elapsed.inWholeMilliseconds != result.elapsed.inWholeMilliseconds ||
            solution.provenance != result.provenance ||
            solution.fingerprints != result.fingerprints ||
            solution.fingerprintSchemas != result.fingerprintSchemas ||
            solution.statistics != result.statistics ||
            solution.diagnostics != result.diagnostics
        ) {
            return null
        }
        val expectedFingerprints = mapOf(
            "model" to modelFingerprint,
            "configuration" to configurationFingerprint
        )
        if (expectedFingerprints.any { (key, value) ->
                solution.fingerprints[key] != value || result.fingerprints[key] != value
            }
        ) {
            return null
        }
        val artifactSolverFingerprint = solution.fingerprints["solver"]
        if (artifactSolverFingerprint.isNullOrBlank() ||
            artifactSolverFingerprint != result.fingerprints["solver"]
        ) {
            return null
        }
        return solution
    }

    private fun decodeModel(encoded: String): DecodedModel {
        val canonical = canonicalSnapshotJson(encoded)
            ?: error("CP snapshot canonicalization failed")
        val root = json.parseToJsonElement(canonical).jsonObject
        val bindings = linkedMapOf<VariableId, AbstractVariableItem<*, *>>()
        val variableEntries = root["variables"]?.jsonArray
            ?: error("CP snapshot variables are missing")
        variableEntries.forEach { element ->
            val item = element.jsonObject
            val id = item.string("id")
            val name = item.string("name")
            val typeName = item.string("typeName")
            val variable = if (typeName.contains("Binary", ignoreCase = true)) {
                BinVar(name)
            } else {
                IntVar(name)
            }
            val stableId = VariableId(id)
            if (bindings.containsKey(stableId)) {
                error("Duplicate CP variable ID: $id")
            }
            bindings[stableId] = variable
        }
        val snapshot = ConstraintProgrammingSnapshotCodec.decode(canonical, bindings).value
            ?: error("CP snapshot decoding returned no value")
        val model = ConstraintProgrammingModel(
            name = snapshot.name,
            objectCategory = snapshot.objectCategory,
            identityNamespace = snapshot.identityNamespace,
            identitySchemaVersion = snapshot.identitySchemaVersion
        )
        snapshot.variables.forEach { variable ->
            val binding = bindings[variable.id]
                ?: error("Missing variable binding: ${variable.id.value}")
            check(
                model.registerVariable(
                    id = variable.id,
                    variable = binding,
                    domain = variable.domain,
                    scope = variable.scope,
                    origin = variable.origin,
                    identityProvenance = variable.identityProvenance
                ).ok
            ) {
                "Failed to register CP variable: ${variable.id.value}"
            }
        }
        snapshot.intervals.forEach { interval ->
            check(model.registerInterval(interval).ok) {
                "Failed to register CP interval: ${interval.id.value}"
            }
        }
        snapshot.expressions.forEach { expression ->
            check(model.registerExpression(expression.name, expression.expression).ok) {
                "Failed to register CP expression: ${expression.name}"
            }
        }
        snapshot.constraints.forEach { constraint ->
            check(
                model.addConstraint(
                    constraint = constraint.constraint,
                    id = constraint.id,
                    name = constraint.name,
                    scope = constraint.scope,
                    origin = constraint.origin,
                    identityProvenance = constraint.identityProvenance
                ).ok
            ) {
                "Failed to register CP constraint: ${constraint.id.value}"
            }
        }
        snapshot.objectives.forEach { objective ->
            check(
                model.addObjective(
                    category = objective.category,
                    expression = objective.expression,
                    id = objective.id,
                    name = objective.name,
                    scope = objective.scope,
                    origin = objective.origin,
                    identityProvenance = objective.identityProvenance
                ).ok
            ) {
                "Failed to register CP objective: ${objective.id.value}"
            }
        }
        check(model.validate().ok) { "Decoded CP model validation failed" }
        return DecodedModel(model, snapshot, bindings)
    }

    /** Canonicalize a compatible snapshot before fingerprinting or reconstruction. /
     * 在计算指纹或重建前规范化兼容 snapshot。
     */
    private fun canonicalSnapshotJson(encoded: String): String? {
        return ConstraintProgrammingSnapshotCodec.canonicalize(encoded).value
    }

    /**
     * Rebuild a CP snapshot through the calculator's server-side reconstruction path. /
     * 通过 calculator 服务端的重建路径重新生成 CP snapshot。
     *
     * This internal boundary is used by protocol regression tests to inspect the model after
     * DTO decoding and CP registration, rather than only inspecting the original JSON. /
     * 此内部边界供协议回归测试检查 DTO 解码和 CP 注册后的模型，而不是只检查原始 JSON。
     *
     * @param encoded CP snapshot JSON / CP snapshot JSON
     * @return rebuilt snapshot / 重建后的 snapshot
     */
    internal fun rebuildSnapshot(encoded: String): ConstraintProgrammingModelSnapshot {
        val decoded = decodeModel(encoded)
        return try {
            decoded.model.snapshot().value
                ?: error("Rebuilt CP snapshot is unavailable")
        } finally {
            decoded.model.close()
        }
    }

    private fun decodeDiagnosticList(encoded: String?): List<String>? {
        if (encoded.isNullOrBlank()) {
            return emptyList()
        }
        return runCatching {
            json.decodeFromString<List<String>>(encoded)
        }.getOrNull()?.takeIf { values -> values.all { it.isNotBlank() } }
    }

    /**
     * Revalidate an external CP result against the decoded snapshot.
     * 根据已解码 snapshot 重新复验外部 CP 结果。
     *
     * @param snapshotJson CP snapshot JSON / CP snapshot JSON
     * @param solution 外部进程返回的结果 artifact / Result artifact returned by the external process
     * @return 结果是否通过变量、值域、约束、interval、目标和状态复验 / Whether the result passes semantic validation
     */
    internal fun validateExternalSolution(
        snapshotJson: String,
        solution: SerializedSolution
    ): Boolean {
        return runCatching {
            val decoded = decodeModel(snapshotJson)
            val diagnosticsMatchStatus = solution.problemStatus != RemoteProblemStatus.FEASIBLE ||
                solution.diagnostics.keys.none { it.startsWith("infeasibility.") }
            val diagnosticsProofMatch = validateDiagnosticProofCombination(solution)
            diagnosticsMatchStatus &&
                validateExternalDiagnostics(decoded.snapshot, solution.diagnostics) &&
                diagnosticsProofMatch &&
                validateSolutionSemantics(decoded.snapshot, solution)
        }.getOrDefault(false)
    }

    private fun validateDiagnosticProofCombination(solution: SerializedSolution): Boolean {
        val source = solution.diagnostics["infeasibility.source"]
        val hasInfeasibilityFields = solution.diagnostics.keys.any { it.startsWith("infeasibility.") }
        val validity = solution.diagnostics["infeasibility.validity"]
        val members = solution.diagnostics["infeasibility.members"]
            ?.let(::decodeDiagnosticList)
            ?: emptyList()
        if (source == null && !hasInfeasibilityFields) {
            return if (solution.solutionPresence == RemoteSolutionPresence.OPTIMAL) {
                solution.proofStatus == RemoteProofStatus.VERIFIED
            } else {
                solution.proofStatus != RemoteProofStatus.VERIFIED
            }
        }
        if (source == "None") {
            return solution.proofStatus != RemoteProofStatus.VERIFIED && members.isEmpty()
        }
        if (validity == "Verified") {
            return solution.proofStatus == RemoteProofStatus.VERIFIED && members.isNotEmpty()
        }
        return solution.proofStatus != RemoteProofStatus.VERIFIED
    }

    /**
     * Validate structured diagnostics against the CP snapshot.
     * 根据 CP snapshot 校验结构化诊断。
     *
     * @param snapshot 已解码的 CP snapshot / Decoded CP snapshot
     * @param diagnostics 外部结果中的诊断字段 / Diagnostics emitted by the external result
     * @return 诊断是否完整且引用有效 / Whether diagnostics are complete and refer to valid members
     */
    internal fun validateExternalDiagnostics(
        snapshot: fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingModelSnapshot,
        diagnostics: Map<String, String>
    ): Boolean {
        val knownInfeasibilityKeys = setOf(
            "infeasibility.source",
            "infeasibility.exactness",
            "infeasibility.completeness",
            "infeasibility.validity",
            "infeasibility.minimality",
            "infeasibility.constraintIds",
            "infeasibility.variableBoundRefs",
            "infeasibility.variableDomainRefs",
            "infeasibility.members",
            "infeasibility.reference",
            "infeasibility.assumptionIds",
            "infeasibility.elapsedMs",
            "infeasibility.unavailable.code",
            "infeasibility.unavailable.category",
            "infeasibility.unavailable.message",
            "infeasibility.verificationChecks",
            "infeasibility.terminationReason"
        )
        val issueKey = Regex("^(warning|error)\\.(\\d+)\\.(code|category|message)$")
        if (diagnostics.keys.any { key ->
                key !in knownInfeasibilityKeys && !issueKey.matches(key)
            }) {
            return false
        }
        val source = diagnostics["infeasibility.source"]
        val hasInfeasibilityFields = diagnostics.keys.any { it.startsWith("infeasibility.") }
        if (source == null) {
            if (hasInfeasibilityFields) {
                return false
            }
        } else if (source !in setOf("NativeIIS", "Farkas", "ConstraintConflict", "ElasticFilter", "DeletionFilter", "None")) {
            return false
        }
        val enumFields = mapOf(
            "infeasibility.exactness" to setOf("Exact", "Irreducible", "Heuristic", "Unknown"),
            "infeasibility.completeness" to setOf("Complete", "Partial", "Unavailable"),
            "infeasibility.validity" to setOf("Verified", "Heuristic", "Unknown"),
            "infeasibility.minimality" to setOf("Irreducible", "Partial", "NotChecked")
        )
        if (enumFields.any { (key, values) -> diagnostics[key]?.let { it !in values } == true }) {
            return false
        }
        val variableIds = snapshot.variables.mapTo(linkedSetOf()) { it.id.value }
        val constraintIds = snapshot.constraints.mapTo(linkedSetOf()) { it.id.value }
        fun list(key: String): List<String>? {
            val encoded = diagnostics[key] ?: return emptyList()
            if (encoded.isBlank()) return null
            return decodeDiagnosticList(encoded)
        }
        val assumptions = list("infeasibility.assumptionIds") ?: return false
        val constraints = list("infeasibility.constraintIds") ?: return false
        val domains = list("infeasibility.variableDomainRefs") ?: return false
        val bounds = list("infeasibility.variableBoundRefs") ?: return false
        val members = list("infeasibility.members") ?: return false
        if (assumptions.any { it !in variableIds } || constraints.any { it !in constraintIds } ||
            domains.any { it !in variableIds }) {
            return false
        }
        if (bounds.any { encoded ->
                val separator = encoded.lastIndexOf(':')
                separator <= 0 || encoded.substring(0, separator) !in variableIds ||
                    encoded.substring(separator + 1) !in setOf("Lower", "Upper")
            }) {
            return false
        }
        if (members.any { member ->
                when {
                    member.startsWith("constraint:") -> member.removePrefix("constraint:") !in constraintIds
                    member.startsWith("domain:") -> member.removePrefix("domain:") !in variableIds
                    member.startsWith("bound:") -> {
                        val encoded = member.removePrefix("bound:")
                        val separator = encoded.lastIndexOf(':')
                        separator <= 0 || encoded.substring(0, separator) !in variableIds ||
                            encoded.substring(separator + 1) !in setOf("Lower", "Upper")
                    }
                    else -> true
                }
            }) {
            return false
        }
        val memberConstraintIds = members.mapNotNullTo(linkedSetOf()) { member ->
            member.takeIf { it.startsWith("constraint:") }?.removePrefix("constraint:")
        }
        val memberDomainRefs = members.mapNotNullTo(linkedSetOf()) { member ->
            member.takeIf { it.startsWith("domain:") }?.removePrefix("domain:")
        }
        val memberBoundRefs = members.mapNotNullTo(linkedSetOf()) { member ->
            member.takeIf { it.startsWith("bound:") }?.removePrefix("bound:")
        }
        if (constraints.toSet() != memberConstraintIds ||
            domains.toSet() != memberDomainRefs ||
            bounds.toSet() != memberBoundRefs
        ) {
            return false
        }
        if (diagnostics["infeasibility.validity"] == "Verified" && members.isEmpty()) {
            return false
        }
        val validity = diagnostics["infeasibility.validity"]
        val minimality = diagnostics["infeasibility.minimality"]
        if (source == "None") {
            if (validity == "Verified" || minimality == "Irreducible" ||
                members.isNotEmpty() || constraints.isNotEmpty() || domains.isNotEmpty() ||
                bounds.isNotEmpty() || assumptions.isNotEmpty()
            ) {
                return false
            }
        } else if (source != null) {
            if (validity.isNullOrBlank() || minimality.isNullOrBlank()) {
                return false
            }
            if (minimality == "Irreducible" && validity != "Verified") {
                return false
            }
        }
        diagnostics["infeasibility.terminationReason"]?.let {
            if (it !in setOf(
                    "COMPLETED", "TIME_LIMIT", "NODE_LIMIT", "ITERATION_LIMIT", "SOLUTION_LIMIT",
                    "OBJECTIVE_LIMIT", "CANCELLED", "INTERRUPTED", "NUMERICAL_FAILURE", "BACKEND_FAILURE"
                )) return false
        }
        diagnostics["infeasibility.verificationChecks"]?.toULongOrNull()
            ?: diagnostics["infeasibility.verificationChecks"]?.let { return false }
        diagnostics["infeasibility.elapsedMs"]?.toLongOrNull()
            ?: diagnostics["infeasibility.elapsedMs"]?.let { return false }
        if (diagnostics["infeasibility.unavailable.code"] != null &&
            (diagnostics["infeasibility.unavailable.category"].isNullOrBlank() ||
                diagnostics["infeasibility.unavailable.message"].isNullOrBlank() ||
                diagnostics["infeasibility.unavailable.category"] !in setOf(
                    "InvalidInput", "Environment", "License", "Numerical", "Callback",
                    "Parsing", "Backend", "Protocol", "Unsupported"
                ))) {
            return false
        }
        for (key in diagnostics.keys.filter { it.startsWith("warning.") || it.startsWith("error.") }) {
            val match = issueKey.matchEntire(key) ?: return false
            val prefix = match.groupValues[1]
            val index = match.groupValues[2]
            val code = diagnostics["$prefix.$index.code"]
            val category = diagnostics["$prefix.$index.category"]
            val message = diagnostics["$prefix.$index.message"]
            if (code.isNullOrBlank() || category !in setOf(
                    "InvalidInput", "Environment", "License", "Numerical", "Callback",
                    "Parsing", "Backend", "Protocol", "Unsupported"
                ) || message.isNullOrBlank()) return false
        }
        return true
    }

    /**
     * Revalidate an external CP checkpoint incumbent against the snapshot.
     * 根据 snapshot 重新复验外部 CP checkpoint 的 incumbent。
     *
     * @param snapshotJson CP snapshot JSON / CP snapshot JSON
     * @param checkpoint portable checkpoint envelope / portable checkpoint envelope
     * @return checkpoint incumbent 是否通过数学复验 / Whether the checkpoint incumbent passes mathematical validation
     */
    internal fun validateExternalCheckpoint(
        snapshotJson: String,
        checkpoint: PortableCheckpointEnvelope
    ): Boolean {
        return runCatching {
            if (checkpoint.benders != null) {
                return@runCatching false
            }
            val canonicalSnapshot = canonicalSnapshotJson(snapshotJson) ?: return@runCatching false
            val normalizedCheckpoint = canonicalizeCheckpoint(checkpoint)
                ?: return@runCatching false
            val canonicalCheckpoint = normalizedCheckpoint.snapshotJson
            val decoded = decodeModel(canonicalSnapshot)
            val incumbent = normalizedCheckpoint.incumbent
            val variableIds = decoded.snapshot.variables.mapTo(linkedSetOf()) { it.id.value }
            val constraintIds = decoded.snapshot.constraints.mapTo(linkedSetOf()) { it.id.value }
            val expectedIdentitySchema = decoded.snapshot.identitySchemaVersion
            val expectedIdentityNamespace = decoded.snapshot.identityNamespace
            if (normalizedCheckpoint.checkpointId.isBlank() ||
                normalizedCheckpoint.modelName.isBlank() ||
                normalizedCheckpoint.identitySchemaVersion.isBlank() ||
                normalizedCheckpoint.identityNamespace.isBlank() ||
                normalizedCheckpoint.modelFingerprint !in setOf(
                    fingerprint(snapshotJson),
                    fingerprint(normalizedCheckpoint.snapshotJson),
                    fingerprint(canonicalCheckpoint),
                    fingerprint(canonicalSnapshot)
                ) ||
                canonicalCheckpoint != canonicalSnapshot ||
                normalizedCheckpoint.modelName != decoded.snapshot.name ||
                normalizedCheckpoint.identitySchemaVersion != expectedIdentitySchema ||
                normalizedCheckpoint.identityNamespace != expectedIdentityNamespace ||
                normalizedCheckpoint.bestBound?.let { it.isBlank() || it.toDoubleOrNull()?.isFinite() != true } == true ||
                normalizedCheckpoint.gap?.let { it.isBlank() || it.toDoubleOrNull()?.isFinite() != true } == true
            ) {
                return@runCatching false
            }
            val evidenceValid = normalizedCheckpoint.assumptions.all { it in variableIds } &&
                normalizedCheckpoint.conflicts.all { conflict ->
                    conflict.validity in setOf("Verified", "Heuristic", "Unknown") &&
                        conflict.minimality in setOf("Irreducible", "Partial", "NotChecked") &&
                        (conflict.validity != "Verified" || conflict.memberIds.isNotEmpty()) &&
                        conflict.assumptionIds.all { it in variableIds } &&
                        conflict.memberIds.all { member ->
                            when {
                                member.startsWith("constraint:") -> member.removePrefix("constraint:") in constraintIds
                                member.startsWith("domain:") -> member.removePrefix("domain:") in variableIds
                                member.startsWith("bound:") -> {
                                    val separator = member.removePrefix("bound:").lastIndexOf(':')
                                    val id = member.removePrefix("bound:").substring(0, separator.coerceAtLeast(0))
                                    val side = member.substringAfterLast(':', "")
                                    id in variableIds && side in setOf("Lower", "Upper")
                                }
                                else -> false
                            }
                        }
                }
            if (!evidenceValid) {
                return@runCatching false
            }
            if (incumbent == null) {
                true
            } else {
                val objective = incumbent.objective?.let { value ->
                    value.toLongOrNull() ?: return@runCatching false
                }
                val solution = SerializedSolution(
                    feasible = true,
                    optimal = false,
                    objectiveValueInt64 = objective,
                    variableValuesById = incumbent.valuesById,
                    intervalValues = incumbent.intervalsById.mapValues { (_, value) ->
                        SerializedIntervalValue(
                            start = value.start,
                            size = value.size,
                            end = value.end,
                            present = value.present
                        )
                    },
                    problemStatus = RemoteProblemStatus.FEASIBLE,
                    solutionPresence = RemoteSolutionPresence.INCUMBENT,
                    proofStatus = RemoteProofStatus.NONE,
                    terminationReason = RemoteTerminationReason.COMPLETED,
                    schemaVersion = CP_RESULT_SCHEMA_VERSION
                )
                validateSolutionSemantics(decoded.snapshot, solution)
            }
        }.getOrDefault(false)
    }

    private fun validateSolutionSemantics(
        snapshot: fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingModelSnapshot,
        solution: SerializedSolution
    ): Boolean {
        val variableIds = snapshot.variables.mapTo(linkedSetOf()) { it.id.value }
        val intervalIds = snapshot.intervals.mapTo(linkedSetOf()) { it.id.value }
        val hasIncumbent = solution.feasible || solution.solutionPresence != RemoteSolutionPresence.NONE
        if (!hasIncumbent) {
            val validNonIncumbentProof = when (solution.problemStatus) {
                RemoteProblemStatus.UNKNOWN -> solution.proofStatus != RemoteProofStatus.VERIFIED
                RemoteProblemStatus.INFEASIBLE,
                RemoteProblemStatus.UNBOUNDED,
                RemoteProblemStatus.INFEASIBLE_OR_UNBOUNDED ->
                    solution.proofStatus == RemoteProofStatus.CLAIMED ||
                        solution.proofStatus == RemoteProofStatus.VERIFIED

                RemoteProblemStatus.FEASIBLE -> false
                null -> false
            }
            return solution.variableValuesById.isEmpty() &&
                solution.intervalValues.isEmpty() &&
                solution.solutionPresence == RemoteSolutionPresence.NONE &&
                !solution.optimal &&
                solution.objectiveValue == null &&
                solution.objectiveValueInt64 == null &&
                validNonIncumbentProof
        }
        if (solution.problemStatus != RemoteProblemStatus.FEASIBLE ||
            !solution.feasible ||
            solution.solutionPresence == RemoteSolutionPresence.NONE ||
            solution.optimal != (solution.solutionPresence == RemoteSolutionPresence.OPTIMAL) ||
            solution.variableValuesById.keys != variableIds ||
            solution.intervalValues.keys != intervalIds
        ) {
            return false
        }
        val values = solution.variableValuesById
            .mapKeys { VariableId(it.key) }
            .mapValues { Int64(it.value) }
        if (values.any { (id, value) -> snapshot.variable(id)?.domain?.contains(value) != true }) {
            return false
        }
        if (snapshot.constraints.any { constraint ->
                val satisfied = constraint.constraint.isSatisfied(values)
                satisfied.failed || satisfied.value != true
            }) {
            return false
        }
        if (snapshot.intervals.any { interval ->
                val evaluated = interval.evaluate(values)
                val supplied = solution.intervalValues[interval.id.value]
                evaluated.failed || supplied == null ||
                    supplied.start != evaluated.value!!.start.toLong() ||
                    supplied.size != evaluated.value!!.size.toLong() ||
                    supplied.end != evaluated.value!!.end.toLong() ||
                    supplied.present != evaluated.value!!.present
            }) {
            return false
        }
        val objective = snapshot.objectives.firstOrNull()?.expression
        val evaluatedObjective = objective?.evaluate(values)
        if (evaluatedObjective?.failed == true) {
            return false
        }
        if (objective == null) {
            return solution.objectiveValue == null && solution.objectiveValueInt64 == null
        }
        if (solution.objectiveValue != null) {
            return false
        }
        val expected = evaluatedObjective!!.value!!.toString()
        val actual = solution.objectiveValueInt64?.toString() ?: solution.objectiveValue?.toString()
        // A backend failure may still carry a mathematically valid incumbent while omitting the
        // objective field.  The checkpoint exporter recomputes that objective from the snapshot;
        // all other terminal states must carry the exact value. / 后端失败仍可能带有数学有效
        // incumbent 但缺少目标字段；checkpoint 导出器会从 snapshot 重算，其他终态必须携带精确值。
        if (actual == null) {
            return solution.terminationReason == RemoteTerminationReason.BACKEND_FAILURE
        }
        if (actual != expected) {
            return false
        }
        if (solution.solutionPresence == RemoteSolutionPresence.OPTIMAL &&
            solution.proofStatus != RemoteProofStatus.VERIFIED
        ) {
            return false
        }
        return true
    }

    private suspend fun solve(
        model: ConstraintProgrammingModel,
        payload: SolvePayload,
        quantumMs: Long?,
        cancellationToken: CancellationToken?,
        checkpoint: ConstraintProgrammingSolution?
    ): ConstraintProgrammingSolverOutput {
        val config = solveConfiguration(payload, quantumMs)
        val solutionLimit = config.solutionLimit
        val solver = ScipConstraintProgrammingSolver()
        val options = ConstraintProgrammingSolveOptions(
            timeLimit = config.timeLimit,
            solutionLimit = solutionLimit?.toULong()?.let(::UInt64),
            threadCount = config.threads,
            cancellationToken = cancellationToken,
            relativeObjectiveGap = config.mipGapTolerance,
            configurationFingerprint = fingerprint(configurationCanonical(semanticConfiguration(payload)))
        )
        val session = solver.createSession(model, options).value
            ?: error("SCIP CP session creation returned no value")
        val hint = checkpoint
        return try {
            session.solve(hints = hint).value ?: error("SCIP CP solve returned no output")
        } finally {
            session.close()
        }
    }

    private fun validateCheckpoint(
        payload: SolvePayload,
        decoded: DecodedModel,
        encodedSnapshot: String,
        rawCheckpoint: PortableCheckpointEnvelope?,
        configurationFingerprintOverride: String? = null,
        taskId: String,
        sliceId: String
    ): Ret<ConstraintProgrammingSolution?> {
        if (rawCheckpoint == null) {
            return ok(null)
        }
        if (rawCheckpoint.benders != null) {
            return Failed(
                fuookami.ospf.kotlin.utils.error.ErrorCode.ORSolutionInvalid,
                "plain CP executor 不支持 Benders checkpoint / Plain CP executor does not support Benders checkpoints"
            )
        }
        if (rawCheckpoint.sourceFormat == "v2") {
            val encodedCheckpoint = runCatching {
                json.encodeToString(PortableCheckpointEnvelope.serializer(), rawCheckpoint)
            }.getOrNull()
            val verifiedCheckpoint = encodedCheckpoint?.let(PortableCheckpointCodec::decodeOrNull)
            if (verifiedCheckpoint != rawCheckpoint) {
                return Failed(
                    fuookami.ospf.kotlin.utils.error.ErrorCode.ORSolutionInvalid,
                    "v2 checkpoint 完整性摘要无效 / V2 checkpoint integrity digest is invalid"
                )
            }
        }
        if (rawCheckpoint.sourceFormat == "v2" && rawCheckpoint.checkpointId == "legacy-v1") {
            return Failed(
                fuookami.ospf.kotlin.utils.error.ErrorCode.ORSolutionInvalid,
                "v2 checkpoint 不能伪装为 legacy-v1 / A v2 checkpoint must not masquerade as legacy-v1"
            )
        }
        if (rawCheckpoint.sourceFormat == "legacy-v1" && rawCheckpoint.checkpointId != "legacy-v1") {
            return Failed(
                fuookami.ospf.kotlin.utils.error.ErrorCode.ORSolutionInvalid,
                "legacy checkpoint 来源标识无效 / Legacy checkpoint source identifier is invalid"
            )
        }
        if (rawCheckpoint.sourceFormat == "v2" && rawCheckpoint.migratedFromLegacy &&
            rawCheckpoint.checkpointId != "legacy-v1-migrated"
        ) {
            return Failed(
                fuookami.ospf.kotlin.utils.error.ErrorCode.ORSolutionInvalid,
                "legacy 迁移 checkpoint 标识无效 / Migrated legacy checkpoint identifier is invalid"
            )
        }
        val legacyV1 = rawCheckpoint.sourceFormat == "legacy-v1"
        if (rawCheckpoint.sourceFormat == "v2" && rawCheckpoint.runId != taskId) {
            return Failed(
                fuookami.ospf.kotlin.utils.error.ErrorCode.ORSolutionInvalid,
                "checkpoint 运行归属与当前 task 不一致 / Checkpoint run ownership does not match the current task"
            )
        }
        if (rawCheckpoint.sourceFormat == "v2" && rawCheckpoint.attemptId.isNullOrBlank()) {
            return Failed(
                fuookami.ospf.kotlin.utils.error.ErrorCode.ORSolutionInvalid,
                "checkpoint 缺少历史 attempt 归属 / Checkpoint is missing its source attempt ownership"
            )
        }
        val checkpoint = canonicalizeCheckpoint(rawCheckpoint)
            ?: return Failed(
                fuookami.ospf.kotlin.utils.error.ErrorCode.ORSolutionInvalid,
                "checkpoint snapshot 规范化失败 / Checkpoint snapshot canonicalization failed"
            )
        val legacyV2WithoutConfiguration = checkpoint.schemaVersion == "2.0" &&
            checkpoint.configurationFingerprint.isNullOrBlank()
        val migratedLegacyV2 = checkpoint.sourceFormat == "v2" && checkpoint.migratedFromLegacy
        if (checkpoint.modelName != decoded.snapshot.name) {
            return Failed(
                fuookami.ospf.kotlin.utils.error.ErrorCode.ORSolutionInvalid,
                "checkpoint 模型名称与当前 snapshot 不一致 / Checkpoint model name disagrees with the current snapshot"
            )
        }
        if (checkpoint.modelFingerprint != fingerprint(encodedSnapshot)) {
            return Failed(
                fuookami.ospf.kotlin.utils.error.ErrorCode.ORSolutionInvalid,
                "checkpoint 模型指纹不匹配 / Checkpoint model fingerprint mismatch"
            )
        }
        val variableIds = decoded.snapshot.variables.mapTo(linkedSetOf()) { it.id.value }
        val constraintIds = decoded.snapshot.constraints.mapTo(linkedSetOf()) { it.id.value }
        if (checkpoint.assumptions.any { it !in variableIds }) {
            return Failed(
                fuookami.ospf.kotlin.utils.error.ErrorCode.ORSolutionInvalid,
                "checkpoint assumption 引用了未知变量 / Checkpoint assumption references an unknown variable"
            )
        }
        for (conflict in checkpoint.conflicts) {
            if (conflict.validity !in setOf("Verified", "Heuristic", "Unknown") ||
                conflict.minimality !in setOf("Irreducible", "Partial", "NotChecked")
            ) {
                return Failed(
                    fuookami.ospf.kotlin.utils.error.ErrorCode.ORSolutionInvalid,
                    "checkpoint conflict 证据枚举无效 / Checkpoint conflict evidence enum is invalid"
                )
            }
            if (conflict.assumptionIds.any { it !in variableIds }) {
                return Failed(
                    fuookami.ospf.kotlin.utils.error.ErrorCode.ORSolutionInvalid,
                    "checkpoint conflict 引用了未知 assumption / Checkpoint conflict references an unknown assumption"
                )
            }
            for (member in conflict.memberIds) {
                when {
                    member.startsWith("constraint:") -> {
                        if (member.removePrefix("constraint:") !in constraintIds) {
                            return Failed(
                                fuookami.ospf.kotlin.utils.error.ErrorCode.ORSolutionInvalid,
                                "checkpoint conflict 引用了未知约束 / Checkpoint conflict references an unknown constraint"
                            )
                        }
                    }
                    member.startsWith("domain:") -> {
                        if (member.removePrefix("domain:") !in variableIds) {
                            return Failed(
                                fuookami.ospf.kotlin.utils.error.ErrorCode.ORSolutionInvalid,
                                "checkpoint conflict 引用了未知变量域 / Checkpoint conflict references an unknown variable domain"
                            )
                        }
                    }
                    member.startsWith("bound:") -> {
                        val encodedMember = member.removePrefix("bound:")
                        val separator = encodedMember.lastIndexOf(':')
                        val variableId = if (separator > 0) encodedMember.substring(0, separator) else ""
                        val side = if (separator > 0) encodedMember.substring(separator + 1) else ""
                        if (variableId !in variableIds || side !in setOf("Lower", "Upper")) {
                            return Failed(
                                fuookami.ospf.kotlin.utils.error.ErrorCode.ORSolutionInvalid,
                                "checkpoint conflict 边界成员无效 / Checkpoint conflict bound member is invalid"
                            )
                        }
                    }
                    else -> {
                        return Failed(
                            fuookami.ospf.kotlin.utils.error.ErrorCode.ORSolutionInvalid,
                            "checkpoint conflict 成员类型未知 / Checkpoint conflict member type is unknown"
                        )
                    }
                }
            }
        }
        if (fingerprint(checkpoint.snapshotJson) != checkpoint.modelFingerprint ||
            checkpoint.snapshotJson != encodedSnapshot
        ) {
            return Failed(
                fuookami.ospf.kotlin.utils.error.ErrorCode.ORSolutionInvalid,
                "checkpoint 内嵌 snapshot 与当前模型不一致 / Checkpoint embedded snapshot disagrees with the current model"
            )
        }
        val expectedConfigurationFingerprint = configurationFingerprintOverride
            ?: fingerprint(configurationCanonical(semanticConfiguration(payload)))
        val legacyV2PublishedConfiguration = checkpoint.schemaVersion == "2.0" &&
            isLegacyV2ConfigurationFingerprint(payload, checkpoint.configurationFingerprint)
        val expectedSolverFingerprint = scipRuntimeFingerprint()
        val legacySolverFingerprint = isLegacyV2SolverFingerprint(checkpoint.solverFingerprint)
        val legacyV2Solver = checkpoint.schemaVersion == "2.0" &&
            legacySolverFingerprint
        val migratedConfiguration = legacyV1 || legacyV2WithoutConfiguration || legacyV2PublishedConfiguration
        val migratedSolver = migratedLegacyV2 || legacyV2Solver ||
            (legacyV1 && (checkpoint.solverFingerprint == null || legacySolverFingerprint))
        if (checkpoint.configurationFingerprint != expectedConfigurationFingerprint &&
            !(legacyV1 && checkpoint.configurationFingerprint == null) &&
            !legacyV2WithoutConfiguration &&
            !legacyV2PublishedConfiguration
        ) {
            return Failed(
                fuookami.ospf.kotlin.utils.error.ErrorCode.ORSolutionInvalid,
                "checkpoint 配置指纹不匹配 / Checkpoint configuration fingerprint mismatch"
            )
        }
        if (!migratedSolver && checkpoint.solverFingerprint != expectedSolverFingerprint) {
            return Failed(
                fuookami.ospf.kotlin.utils.error.ErrorCode.ORSolutionInvalid,
                "checkpoint 求解器运行时身份不匹配 / Checkpoint solver runtime identity mismatch"
            )
        }
        val envelope = fuookami.ospf.kotlin.core.solver.constraint_programming.ConstraintProgrammingCheckpointEnvelope(
            schemaVersion = checkpoint.schemaVersion,
            sourceFormat = checkpoint.sourceFormat,
            migratedFromLegacy = checkpoint.migratedFromLegacy,
            checkpointId = checkpoint.checkpointId,
            identitySchemaVersion = checkpoint.identitySchemaVersion,
            identityNamespace = checkpoint.identityNamespace,
            modelName = checkpoint.modelName,
            modelFingerprint = checkpoint.modelFingerprint,
            configurationFingerprint = checkpoint.configurationFingerprint,
            solverFingerprint = checkpoint.solverFingerprint,
            runId = checkpoint.runId,
            attemptId = checkpoint.attemptId,
            parentCheckpointId = checkpoint.parentCheckpointId,
            createdAtEpochMs = checkpoint.createdAtEpochMs,
            snapshotJson = checkpoint.snapshotJson,
            incumbent = checkpoint.incumbent?.let { incumbent ->
                fuookami.ospf.kotlin.core.solver.constraint_programming.PortableConstraintProgrammingIncumbent(
                    valuesById = incumbent.valuesById,
                    intervalsById = incumbent.intervalsById.mapValues { (_, value) ->
                        fuookami.ospf.kotlin.core.solver.constraint_programming.PortableConstraintProgrammingIntervalValue(
                            start = value.start,
                            size = value.size,
                            end = value.end,
                            present = value.present
                        )
                    },
                    objective = incumbent.objective
                )
            },
            bestBound = checkpoint.bestBound.takeUnless { migratedConfiguration },
            gap = checkpoint.gap.takeUnless { migratedConfiguration },
            assumptions = checkpoint.assumptions,
            conflicts = checkpoint.conflicts.map { conflict ->
                fuookami.ospf.kotlin.core.solver.constraint_programming.PortableConstraintProgrammingConflict(
                    validity = conflict.validity,
                    minimality = conflict.minimality,
                    memberIds = conflict.memberIds,
                    assumptionIds = conflict.assumptionIds,
                    provenance = conflict.provenance
                )
            },
            integritySha256 = checkpoint.integritySha256
        )
        val normalizedEnvelope = ConstraintProgrammingCheckpointCodec.withIntegrity(envelope)
        return when (val restored = ConstraintProgrammingCheckpointCodec.restore(
            envelope = normalizedEnvelope,
            snapshot = decoded.snapshot,
            expectedConfigurationFingerprint = expectedConfigurationFingerprint.takeUnless { migratedConfiguration },
            expectedSolverFingerprint = expectedSolverFingerprint.takeUnless { migratedSolver },
            allowLegacyConfigurationFingerprint = migratedConfiguration,
            allowLegacySolverFingerprint = migratedSolver
        )) {
            is Ok -> ok(restored.value?.incumbent)
            is Failed -> Failed(restored.error)
            is Fatal -> Fatal(restored.errors)
        }
    }

    /** Normalize a checkpoint snapshot before applying core restore checks. /
     * 在交由 core 恢复校验前规范化 checkpoint 内嵌 snapshot。
     */
    private fun canonicalizeCheckpoint(
        checkpoint: PortableCheckpointEnvelope
    ): PortableCheckpointEnvelope? {
        val snapshotJson = canonicalSnapshotJson(checkpoint.snapshotJson) ?: return null
        val snapshotRoot = runCatching {
            json.parseToJsonElement(snapshotJson).jsonObject
        }.getOrNull() ?: return null
        val normalized = checkpoint.copy(
            identitySchemaVersion = snapshotRoot["identitySchemaVersion"]?.jsonPrimitive?.content
                ?: checkpoint.identitySchemaVersion,
            identityNamespace = snapshotRoot["identityNamespace"]?.jsonPrimitive?.content
                ?: checkpoint.identityNamespace,
            modelName = snapshotRoot["name"]?.jsonPrimitive?.content ?: checkpoint.modelName,
            modelFingerprint = fingerprint(snapshotJson),
            snapshotJson = snapshotJson,
            integritySha256 = ""
        )
        if (normalized.sourceFormat != "v2") {
            return normalized
        }
        return runCatching {
            PortableCheckpointCodec.decodeOrNull(PortableCheckpointCodec.encode(normalized))
        }.getOrNull()
    }

    private fun semanticConfiguration(payload: SolvePayload): SolverConfig {
        val config = payload.config ?: SolverConfig(timeLimit = null)
        return config.copy(
            timeLimit = payload.config?.timeLimit ?: payload.taskMeta.timeLimit,
            solutionLimit = payload.config?.solutionLimit ?: payload.taskMeta.solutionLimit,
            threads = config.threads ?: DEFAULT_SCIP_THREADS
        )
    }

    private fun solveConfiguration(payload: SolvePayload, quantumMs: Long?): SolverConfig {
        val config = semanticConfiguration(payload)
        val quantumLimit = quantumMs?.coerceAtLeast(1L)?.milliseconds
        val configuredTimeLimit = config.timeLimit
        val effectiveTimeLimit = when {
            configuredTimeLimit == null -> quantumLimit
            quantumLimit == null -> config.timeLimit
            else -> minOf(configuredTimeLimit, quantumLimit)
        }
        return config.copy(timeLimit = effectiveTimeLimit)
    }

    private fun legacyV2Configuration(payload: SolvePayload): SolverConfig {
        val config = payload.config ?: SolverConfig(timeLimit = null)
        return config.copy(
            timeLimit = payload.config?.timeLimit ?: payload.taskMeta.timeLimit,
            solutionLimit = payload.config?.solutionLimit ?: payload.taskMeta.solutionLimit
        )
    }

    internal fun legacyV2ConfigurationFingerprints(payload: SolvePayload): Set<String> {
        return buildSet {
            payload.config?.let { config ->
                add(fingerprint(legacyDelimiterConfigurationCanonical(config)))
                add(fingerprint(configurationCanonical(config)))
            }
            add(fingerprint(configurationCanonical(legacyV2Configuration(payload))))
        }
    }

    /**
     * Check whether a checkpoint carries one published V2 configuration fingerprint.
     * 判断 checkpoint 是否携带已发布的 V2 配置指纹。
     */
    internal fun isLegacyV2ConfigurationFingerprint(
        payload: SolvePayload,
        candidate: String?
    ): Boolean {
        return candidate != null && candidate in legacyV2ConfigurationFingerprints(payload)
    }

    /**
     * Check whether a V2 solver fingerprint is an explicitly supported legacy value.
     * 判断 V2 求解器指纹是否属于明确支持的 legacy 值。
     */
    internal fun isLegacyV2SolverFingerprint(candidate: String?): Boolean {
        return candidate == legacyScipSolverFingerprint() ||
            candidate in scipRuntimeFingerprintLegacyV1Candidates()
    }

    private fun legacyDelimiterConfigurationCanonical(config: SolverConfig): String {
        return buildString {
            append(config.timeLimitMs ?: "")
            append('|').append(config.solutionLimit ?: "")
            append('|').append(config.mipGapTolerance ?: "")
            append('|').append(config.threads ?: "")
            config.solverParams.toSortedMap().forEach { (key, value) ->
                append('|').append(key).append('=').append(value)
            }
        }
    }

    private fun legacyScipSolverFingerprint(): String {
        return fingerprint("scip-cp")
    }

    private fun configurationCanonical(config: SolverConfig): String {
        return buildString {
            appendCanonical("timeLimitMs", config.timeLimitMs?.toString())
            appendCanonical("solutionLimit", config.solutionLimit?.toString())
            appendCanonical("mipGapTolerance", config.mipGapTolerance?.toString())
            appendCanonical("threads", config.threads?.toString())
            config.solverParams.toSortedMap().forEach { (key, value) ->
                appendCanonical("solverParam.key", key)
                appendCanonical("solverParam.value", value)
            }
        }
    }

    private fun StringBuilder.appendCanonical(name: String, value: String?) {
        append(name.length)
            .append(':')
            .append(name)
            .append(value?.length ?: -1)
            .append(':')
            .append(value ?: "")
    }

    private fun validateConfiguration(payload: SolvePayload): String? {
        payload.extension["remote-solver.config.decode-error"]?.let { error ->
            return "CP 求解配置解码失败：$error / CP solver configuration decoding failed: $error"
        }
        val config = semanticConfiguration(payload)
        if (config.timeLimit?.isNegative() == true) {
            return "CP timeLimit 不得为负 / CP timeLimit must not be negative"
        }
        config.solutionLimit?.let { solutionLimit ->
            if (solutionLimit <= 0) {
                return "CP solutionLimit 必须为正 / CP solutionLimit must be positive"
            }
        }
        config.threads?.let { threads ->
            if (threads <= 0) {
                return "CP threads 必须为正 / CP threads must be positive"
            }
        }
        config.mipGapTolerance?.toDouble()?.let { gap ->
            if (!gap.isFinite() || gap < 0.0 || gap > 1.0) {
                return "CP mipGapTolerance 必须位于 [0, 1] / CP mipGapTolerance must be in [0, 1]"
            }
        }
        if (config.solverParams.isNotEmpty()) {
            return "SCIP CP 暂不支持 solverParams，必须显式移除或由后端适配 / " +
                "SCIP CP does not support solverParams yet; remove them or provide a backend adapter"
        }
        return null
    }

    private fun canonicalObjective(value: Flt64?): String? {
        val number = value?.toDouble() ?: return null
        if (!number.isFinite()) {
            return value.toString()
        }
        val integral = number.toLong()
        return if (integral.toDouble() == number) {
            integral.toString()
        } else {
            value.toString()
        }
    }

    private fun recomputeObjective(
        snapshot: fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingModelSnapshot,
        solution: SerializedSolution
    ): String? {
        val expression = snapshot.objectives.firstOrNull()?.expression ?: return null
        val values = solution.variableValuesById
            .mapKeys { VariableId(it.key) }
            .mapValues { Int64(it.value) }
        val evaluated = expression.evaluate(values)
        return if (evaluated.failed) {
            null
        } else {
            evaluated.value?.toString()
        }
    }

    private fun toExecution(
        output: ConstraintProgrammingSolverOutput,
        decoded: DecodedModel,
        elapsed: Duration,
        encoded: String,
        runId: String,
        attemptId: String
    ): ExecutionArtifact {
        return when (output) {
            is ConstraintProgrammingFeasibleOutput -> {
                val report = output.report
                val exactObjective = decoded.snapshot.objectives.firstOrNull()?.let { objective ->
                    when (val evaluated = objective.expression.evaluate(output.solution.values)) {
                        is Ok -> evaluated.value
                        is Failed -> error(evaluated.error.message)
                        is Fatal -> error(evaluated.errors.joinToString { it.message })
                    }
                }
                val objectiveFields = exactCpObjectiveFields(exactObjective)
                val presence = report?.solutionPresence ?: if (output.proofStatus.name == "Verified") {
                    SolutionPresence.Optimal
                } else {
                    SolutionPresence.Incumbent
                }
                val proofStatus = output.proofStatus.toRemoteProofStatus()
                val terminationReason = report?.terminationReason?.toRemoteReason()
                    ?: RemoteTerminationReason.COMPLETED
                val serialized = SerializedSolution(
                    feasible = true,
                    optimal = presence == SolutionPresence.Optimal,
                    objectiveValue = objectiveFields.objectiveValue,
                    objectiveValueInt64 = objectiveFields.objectiveValueInt64,
                    gap = report?.statistics?.gap,
                    variableValuesById = decoded.snapshot.variables.associate { variable ->
                        variable.id.value to output.solution.values.getValue(variable.id).toLong()
                    },
                    intervalValues = output.solution.intervals.mapKeys { it.key.value }.mapValues {
                        SerializedIntervalValue(
                            start = it.value.start.toLong(),
                            size = it.value.size.toLong(),
                            end = it.value.end.toLong(),
                            present = it.value.present
                        )
                    },
                    problemStatus = RemoteProblemStatus.FEASIBLE,
                    solutionPresence = when (presence) {
                        SolutionPresence.Optimal -> RemoteSolutionPresence.OPTIMAL
                        else -> RemoteSolutionPresence.INCUMBENT
                    },
                    proofStatus = proofStatus,
                    terminationReason = terminationReason,
                    schemaVersion = CP_RESULT_SCHEMA_VERSION,
                    elapsed = elapsed,
                    provenance = serializeProvenance(report),
                    fingerprints = serializeFingerprints(report, encoded),
                    fingerprintSchemas = serializeFingerprintSchemas(report),
                    statistics = serializeStatistics(report, elapsed, output.bestBound),
                    diagnostics = serializeDiagnostics(report),
                    runId = runId,
                    attemptId = attemptId
                )
                ExecutionArtifact(
                    result = SolveResult(
                    feasible = true,
                    optimal = serialized.optimal,
                    objectiveValue = objectiveFields.objectiveValue,
                    objectiveValueInt64 = objectiveFields.objectiveValueInt64,
                    gap = serialized.gap,
                    elapsed = elapsed,
                    schemaVersion = CP_RESULT_SCHEMA_VERSION,
                    problemStatus = RemoteProblemStatus.FEASIBLE,
                    terminationReason = terminationReason,
                    solutionPresence = serialized.solutionPresence ?: RemoteSolutionPresence.INCUMBENT,
                    proofStatus = proofStatus,
                    provenance = serialized.provenance,
                    fingerprints = serialized.fingerprints,
                    fingerprintSchemas = serialized.fingerprintSchemas,
                    statistics = serialized.statistics,
                    diagnostics = serialized.diagnostics,
                    runId = runId,
                    attemptId = attemptId,
                    message = "SCIP CP completed"
                    ),
                    solution = serialized
                )
            }

            is ConstraintProgrammingInfeasibleOutput -> {
                val report = output.report
                val serialized = SerializedSolution.infeasible("SCIP CP proved infeasible").copy(
                    problemStatus = RemoteProblemStatus.INFEASIBLE,
                    solutionPresence = RemoteSolutionPresence.NONE,
                    elapsed = elapsed,
                    proofStatus = output.proofStatus.toRemoteProofStatus(),
                    terminationReason = output.report?.terminationReason?.toRemoteReason()
                        ?: RemoteTerminationReason.COMPLETED,
                    schemaVersion = CP_RESULT_SCHEMA_VERSION,
                    provenance = serializeProvenance(report),
                    fingerprints = serializeFingerprints(report, encoded),
                    fingerprintSchemas = serializeFingerprintSchemas(report),
                    statistics = serializeStatistics(report, elapsed, null),
                    diagnostics = serializeDiagnostics(report),
                    runId = runId,
                    attemptId = attemptId
                )
                ExecutionArtifact(
                    result = SolveResult(
                    feasible = false,
                    optimal = false,
                    objectiveValue = null,
                    gap = serialized.gap,
                    elapsed = elapsed,
                    schemaVersion = CP_RESULT_SCHEMA_VERSION,
                    problemStatus = RemoteProblemStatus.INFEASIBLE,
                    terminationReason = output.report?.terminationReason?.toRemoteReason()
                        ?: RemoteTerminationReason.COMPLETED,
                    solutionPresence = RemoteSolutionPresence.NONE,
                    proofStatus = output.proofStatus.toRemoteProofStatus(),
                    provenance = serialized.provenance,
                    fingerprints = serialized.fingerprints,
                    fingerprintSchemas = serialized.fingerprintSchemas,
                    statistics = serialized.statistics,
                    diagnostics = serialized.diagnostics,
                    runId = runId,
                    attemptId = attemptId,
                    message = "SCIP CP proved infeasible"
                    ),
                    solution = serialized
                )
            }

            is ConstraintProgrammingUnknownOutput -> {
                val report = output.report
                val serialized = SerializedSolution(
                    feasible = false,
                    optimal = false,
                    gap = report?.statistics?.gap,
                    problemStatus = RemoteProblemStatus.UNKNOWN,
                    solutionPresence = RemoteSolutionPresence.NONE,
                    proofStatus = RemoteProofStatus.NONE,
                    terminationReason = output.terminationReason.toRemoteReason(),
                    elapsed = elapsed,
                    schemaVersion = CP_RESULT_SCHEMA_VERSION,
                    message = output.terminationReason.name,
                    provenance = serializeProvenance(report),
                    fingerprints = serializeFingerprints(report, encoded),
                    fingerprintSchemas = serializeFingerprintSchemas(report),
                    statistics = serializeStatistics(report, elapsed, null),
                    diagnostics = serializeDiagnostics(report),
                    runId = runId,
                    attemptId = attemptId
                )
                ExecutionArtifact(
                    result = SolveResult(
                    feasible = false,
                    optimal = false,
                    objectiveValue = null,
                    gap = serialized.gap,
                    elapsed = elapsed,
                    schemaVersion = CP_RESULT_SCHEMA_VERSION,
                    problemStatus = RemoteProblemStatus.UNKNOWN,
                    terminationReason = output.terminationReason.toRemoteReason(),
                    solutionPresence = RemoteSolutionPresence.NONE,
                    proofStatus = RemoteProofStatus.NONE,
                    provenance = serialized.provenance,
                    fingerprints = serialized.fingerprints,
                    fingerprintSchemas = serialized.fingerprintSchemas,
                    statistics = serialized.statistics,
                    diagnostics = serialized.diagnostics,
                    runId = runId,
                    attemptId = attemptId,
                    message = output.terminationReason.name
                    ),
                    solution = serialized
                )
            }
        }
    }

    private suspend fun writeResultArtifact(
        tenantId: String,
        taskId: String,
        sliceId: String,
        execution: ExecutionArtifact
    ): SolveResult {
        val unsignedSolution = execution.solution.copy(artifactDigest = null)
        val unsignedBytes = json.encodeToString(
            SerializedSolution.serializer(),
            unsignedSolution
        ).encodeToByteArray()
        val artifactDigest = fingerprint(unsignedBytes.decodeToString())
        val solution = execution.solution.copy(artifactDigest = artifactDigest)
        val ref = objectStoragePort.put(
            path = "$tenantId/result/$taskId/$sliceId",
            bytes = json.encodeToString(SerializedSolution.serializer(), solution).encodeToByteArray(),
            metadata = mapOf(
                "tenantId" to tenantId,
                "taskId" to taskId,
                "sliceId" to sliceId,
                "contentType" to "application/json",
                "schemaVersion" to execution.result.schemaVersion,
                "artifactDigest" to artifactDigest
            )
        )
        return execution.result.copy(
            resultRef = ref,
            artifactDigest = artifactDigest,
            elapsed = solution.elapsed
        )
    }

    private fun serializeProvenance(report: SolveReport<Int64>?): Map<String, String> {
        val provenance = report?.provenance ?: return emptyMap()
        return buildMap {
            put("solverId", provenance.descriptor.solverId)
            put("backend", provenance.descriptor.backendName)
            provenance.descriptor.backendVersion?.let { put("backendVersion", it) }
            provenance.descriptor.pluginVersion?.let { put("pluginVersion", it) }
            provenance.nativeVersion?.let { put("nativeVersion", it) }
            provenance.threadCount?.let { put("threads", it.toString()) }
            provenance.randomSeed?.let { put("randomSeed", it.toString()) }
            provenance.deterministic?.let { put("deterministic", it.toString()) }
            provenance.effectiveParameters.forEach { (key, value) -> put("parameter.$key", value) }
            provenance.environmentSummary.forEach { (key, value) -> put("environment.$key", value) }
        }
    }

    private fun serializeFingerprints(
        report: SolveReport<Int64>?,
        encodedSnapshot: String
    ): Map<String, String> {
        return buildMap {
            put("model", fingerprint(encodedSnapshot))
            report?.fingerprints?.configuration?.value?.let { put("configuration", it) }
            report?.fingerprints?.solver?.value?.let { put("solver", it) }
        }
    }

    private fun serializeFingerprintSchemas(
        report: SolveReport<Int64>?
    ): Map<String, String> {
        return buildMap {
            // The model fingerprint is always emitted by serializeFingerprints, even when a
            // backend omits it from its report. / model 指纹始终由 serializeFingerprints 写出，
            // 即使后端报告遗漏该字段也必须补齐对应 schema。
            put("model", SolveReport.CURRENT_SCHEMA_VERSION)
            report?.fingerprints?.model?.let { put("model", it.schemaVersion) }
            report?.fingerprints?.configuration?.let { put("configuration", it.schemaVersion) }
            report?.fingerprints?.solver?.let { put("solver", it.schemaVersion) }
        }
    }

    private fun serializeStatistics(
        report: SolveReport<Int64>?,
        elapsed: Duration,
        fallbackBestBound: fuookami.ospf.kotlin.math.algebra.number.Flt64?
    ): Map<String, String> {
        val statistics = report?.statistics
        return buildMap {
            put("elapsedMs", (statistics?.solveTime?.inWholeMilliseconds ?: elapsed.inWholeMilliseconds).toString())
            statistics?.iterations?.let { put("iterations", it.toString()) }
            statistics?.nodes?.let { put("nodes", it.toString()) }
            (statistics?.bestBound?.toString() ?: fallbackBestBound?.toString())?.let {
                put("bestBound", it)
            }
            statistics?.gap?.toString()?.let { put("gap", it) }
        }
    }

    private fun serializeDiagnostics(report: SolveReport<Int64>?): Map<String, String> {
        val diagnostics = report?.diagnostics ?: return emptyMap()
        val evidence = diagnostics.infeasibilityEvidence
        return buildMap {
            if (evidence != null) {
                put("infeasibility.source", evidence.source.name)
                put("infeasibility.exactness", evidence.exactness.name)
                put("infeasibility.completeness", evidence.completeness.name)
                put("infeasibility.validity", evidence.validity.name)
                put("infeasibility.minimality", evidence.minimality.name)
                put("infeasibility.constraintIds", encodeDiagnosticList(evidence.constraintIds.map { it.value }))
                put(
                    "infeasibility.variableBoundRefs",
                    encodeDiagnosticList(evidence.variableBoundRefs.map { "${it.variableId.value}:${it.side.name}" })
                )
                put(
                    "infeasibility.variableDomainRefs",
                    encodeDiagnosticList(evidence.variableDomainRefs.map { it.variableId.value })
                )
                put(
                    "infeasibility.members",
                    encodeDiagnosticList(evidence.members.map { member ->
                        when (member) {
                            is InfeasibilityMember.Constraint -> "constraint:${member.id.value}"
                            is InfeasibilityMember.VariableBound ->
                                "bound:${member.ref.variableId.value}:${member.ref.side.name}"

                            is InfeasibilityMember.VariableDomain -> "domain:${member.ref.variableId.value}"
                        }
                    })
                )
                evidence.reference?.let { put("infeasibility.reference", it) }
                if (evidence.assumptionIds.isNotEmpty()) {
                    put(
                        "infeasibility.assumptionIds",
                        encodeDiagnosticList(evidence.assumptionIds.map { it.value })
                    )
                }
                evidence.elapsed?.let { put("infeasibility.elapsedMs", it.inWholeMilliseconds.toString()) }
                evidence.unavailableReason?.let { issue ->
                    put("infeasibility.unavailable.code", issue.code)
                    put("infeasibility.unavailable.category", issue.category.name)
                    put("infeasibility.unavailable.message", issue.message)
                }
                evidence.verificationChecks?.let { put("infeasibility.verificationChecks", it.toString()) }
                evidence.terminationReason?.let { put("infeasibility.terminationReason", it.name) }
            }
            diagnostics.warnings.forEachIndexed { index, issue ->
                put("warning.$index.code", issue.code)
                put("warning.$index.category", issue.category.name)
                put("warning.$index.message", issue.message)
            }
            diagnostics.errors.forEachIndexed { index, issue ->
                put("error.$index.code", issue.code)
                put("error.$index.category", issue.category.name)
                put("error.$index.message", issue.message)
            }
        }
    }

    private fun encodeDiagnosticList(values: List<String>): String {
        return json.encodeToString(values)
    }

    private fun failureResult(message: String, taskId: String, sliceId: String): SolveResult {
        return SolveResult(
            feasible = false,
            optimal = false,
            objectiveValue = null,
            gap = null,
            elapsed = Duration.ZERO,
            schemaVersion = CP_RESULT_SCHEMA_VERSION,
            problemStatus = RemoteProblemStatus.UNKNOWN,
            terminationReason = RemoteTerminationReason.BACKEND_FAILURE,
            solutionPresence = RemoteSolutionPresence.NONE,
            runId = taskId,
            attemptId = sliceId,
            message = message
        )
    }

    private fun TerminationReason.toRemoteReason(): RemoteTerminationReason {
        return when (this) {
            TerminationReason.Completed -> RemoteTerminationReason.COMPLETED
            TerminationReason.TimeLimit -> RemoteTerminationReason.TIME_LIMIT
            TerminationReason.NodeLimit -> RemoteTerminationReason.NODE_LIMIT
            TerminationReason.IterationLimit -> RemoteTerminationReason.ITERATION_LIMIT
            TerminationReason.SolutionLimit -> RemoteTerminationReason.SOLUTION_LIMIT
            TerminationReason.ObjectiveLimit -> RemoteTerminationReason.OBJECTIVE_LIMIT
            TerminationReason.Cancelled -> RemoteTerminationReason.CANCELLED
            TerminationReason.Interrupted -> RemoteTerminationReason.INTERRUPTED
            TerminationReason.NumericalFailure -> RemoteTerminationReason.NUMERICAL_FAILURE
            TerminationReason.BackendFailure -> RemoteTerminationReason.BACKEND_FAILURE
        }
    }

    private fun RemoteProofStatus.toRemoteProofStatus(): RemoteProofStatus {
        return this
    }

    private fun fuookami.ospf.kotlin.core.solver.report.ProofStatus.toRemoteProofStatus(): RemoteProofStatus {
        return when (this) {
            fuookami.ospf.kotlin.core.solver.report.ProofStatus.None -> RemoteProofStatus.NONE
            fuookami.ospf.kotlin.core.solver.report.ProofStatus.Claimed -> RemoteProofStatus.CLAIMED
            fuookami.ospf.kotlin.core.solver.report.ProofStatus.Verified -> RemoteProofStatus.VERIFIED
        }
    }

    private fun fingerprint(encoded: String): String {
        val digest = java.security.MessageDigest.getInstance("SHA-256")
            .digest(encoded.toByteArray(Charsets.UTF_8))
        return digest.joinToString(separator = "") { byte -> "%02x".format(byte) }
    }

    private fun JsonObject.string(name: String): String {
        return this[name]?.jsonPrimitive?.content
            ?: error("CP snapshot field is missing: $name")
    }

    private data class DecodedModel(
        val model: ConstraintProgrammingModel,
        val snapshot: fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingModelSnapshot,
        val bindings: Map<VariableId, AbstractVariableItem<*, *>>
    )

    private data class ExecutionArtifact(
        val result: SolveResult,
        val solution: SerializedSolution
    )
}
