package fuookami.ospf.framework.remote_solver.adapter.ospf

import java.nio.file.Files
import java.nio.file.Path
import java.security.MessageDigest
import kotlin.time.Duration
import kotlin.time.Duration.Companion.nanoseconds
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.runBlocking
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import org.junit.jupiter.api.Assumptions.assumeTrue
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.algebra.number.Int64
import fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingConstraint
import fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingExpression
import fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingModel
import fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingSnapshotCodec
import fuookami.ospf.kotlin.core.model.constraint_programming.IntegerDomain
import fuookami.ospf.kotlin.core.solver.scip.ScipConstraintProgrammingSolver
import fuookami.ospf.kotlin.core.solver.scip.scipRuntimeFingerprint
import fuookami.ospf.kotlin.core.solver.report.ConstraintId
import fuookami.ospf.kotlin.core.solver.report.ModelElementOrigin
import fuookami.ospf.kotlin.core.solver.report.ObjectiveId
import fuookami.ospf.kotlin.core.solver.report.VariableId
import fuookami.ospf.framework.remote_solver.protocol.domain.ModelData
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectPath
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.PortableCheckpointCodec
import fuookami.ospf.framework.remote_solver.protocol.domain.PortableCheckpointEnvelope
import fuookami.ospf.framework.remote_solver.protocol.domain.PortableConstraintProgrammingBendersState
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteProblemStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteProofStatus
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteSolutionPresence
import fuookami.ospf.framework.remote_solver.protocol.domain.RemoteTerminationReason
import fuookami.ospf.framework.remote_solver.protocol.domain.SerializedSolution
import fuookami.ospf.framework.remote_solver.protocol.domain.SolvePayload
import fuookami.ospf.framework.remote_solver.protocol.domain.SolveResult
import fuookami.ospf.framework.remote_solver.protocol.domain.SolverConfig
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskMeta
import fuookami.ospf.framework.remote_solver.protocol.port.ObjectStoragePort

/** CP calculator regression tests. / CP calculator 回归测试。 */
class OspfCpSnapshotExecutorTest {
    @Test
    fun serverSideCpRebuildPreservesRootIdentityAndCompleteProvenance() {
        val model = ConstraintProgrammingModel(
            name = "remote-identity-rebuild",
            identityNamespace = "remote-fixture",
            identitySchemaVersion = "3.0"
        )
        val variable = fuookami.ospf.kotlin.core.variable.IntVar("x")
        val variableId = VariableId("remote:variable:x")
        val variableProvenance = listOf(
            ModelElementOrigin("variable", "x"),
            ModelElementOrigin("pipeline", "capacity")
        )
        val constraintProvenance = listOf(
            ModelElementOrigin("constraint", "capacity"),
            ModelElementOrigin("pipeline", "capacity")
        )
        val objectiveProvenance = listOf(
            ModelElementOrigin("objective", "cost"),
            ModelElementOrigin("pipeline", "capacity")
        )
        try {
            assertTrue(
                model.registerVariable(
                    id = variableId,
                    variable = variable,
                    domain = IntegerDomain.interval(0, 1).value!!,
                    scope = "STABLE",
                    origin = "legacy/x",
                    identityProvenance = variableProvenance
                ).ok
            )
            val expression = ConstraintProgrammingExpression.Variable(variable)
            assertTrue(
                model.addConstraint(
                    constraint = ConstraintProgrammingConstraint.equal(expression, Int64.zero).value!!,
                    id = ConstraintId("remote:constraint:capacity"),
                    scope = "STABLE",
                    origin = "legacy/capacity",
                    identityProvenance = constraintProvenance
                ).ok
            )
            assertTrue(
                model.minimize(
                    expression = expression,
                    id = ObjectiveId("remote:objective:cost"),
                    scope = "STABLE",
                    origin = "legacy/cost",
                    identityProvenance = objectiveProvenance
                ).ok
            )

            val encoded = ConstraintProgrammingSnapshotCodec.encode(model.snapshot().value!!).value!!
            val rebuilt = OspfCpSnapshotExecutor(RecordingStorage()).rebuildSnapshot(encoded)

            assertEquals("remote-fixture", rebuilt.identityNamespace)
            assertEquals("3.0", rebuilt.identitySchemaVersion)
            assertEquals(
                (variableProvenance + ModelElementOrigin("legacy-origin", "legacy/x"))
                    .distinct()
                    .sortedWith(compareBy({ it.kind }, { it.key })),
                rebuilt.variables.single().identityProvenance
            )
            assertEquals(
                (constraintProvenance + ModelElementOrigin("legacy-origin", "legacy/capacity"))
                    .distinct()
                    .sortedWith(compareBy({ it.kind }, { it.key })),
                rebuilt.constraints.single().identityProvenance
            )
            assertEquals(
                (objectiveProvenance + ModelElementOrigin("legacy-origin", "legacy/cost"))
                    .distinct()
                    .sortedWith(compareBy({ it.kind }, { it.key })),
                rebuilt.objectives.single().identityProvenance
            )
        } finally {
            model.close()
        }
    }

    /**
     * Verify historical scope spelling and omitted provenance are canonicalized before server rebuild.
     * 验证服务端重建前会规范化历史 scope 拼写和省略的 provenance。
     */
    @Test
    fun serverSideCpRebuildCanonicalizesHistoricalIdentityFields() {
        val model = ConstraintProgrammingModel(
            name = "historical-identity-rebuild",
            identityNamespace = "historical-fixture",
            identitySchemaVersion = "1.0"
        )
        val variable = fuookami.ospf.kotlin.core.variable.IntVar("x")
        try {
            assertTrue(
                model.registerVariable(
                    id = VariableId("historical:variable:x"),
                    variable = variable,
                    domain = IntegerDomain.interval(0, 1).value!!,
                    scope = "stable",
                    origin = "legacy/x"
                ).ok
            )
            val encoded = ConstraintProgrammingSnapshotCodec.encode(model.snapshot().value!!).value!!
            val root = Json.parseToJsonElement(encoded).jsonObject.toMutableMap()
            val historicalVariables = root.getValue("variables").jsonArray.map { element ->
                val historical = element.jsonObject.toMutableMap()
                historical.remove("identityProvenance")
                historical["scope"] = JsonPrimitive("STABLE")
                JsonObject(historical)
            }
            root["variables"] = JsonArray(historicalVariables)
            val historical = Json.encodeToString(JsonObject(root))

            val rebuilt = OspfCpSnapshotExecutor(RecordingStorage()).rebuildSnapshot(historical)

            assertEquals("stable", rebuilt.variables.single().scope)
            assertEquals(
                listOf(ModelElementOrigin("legacy-origin", "legacy/x")),
                rebuilt.variables.single().identityProvenance
            )
        } finally {
            model.close()
        }
    }

    /**
     * Verify remote checkpoint validation migrates a historical v2 snapshot before comparison. /
     * 验证远程 checkpoint 校验会在比较前迁移历史 v2 snapshot。
     */
    @Test
    fun remoteCheckpointValidationCanonicalizesHistoricalV2Snapshot() {
        val model = ConstraintProgrammingModel(
            name = "historical-checkpoint-validation",
            identityNamespace = "historical-fixture",
            identitySchemaVersion = "1.0"
        )
        val variable = fuookami.ospf.kotlin.core.variable.IntVar("x")
        try {
            assertTrue(
                model.registerVariable(
                    id = VariableId("historical:variable:x"),
                    variable = variable,
                    domain = IntegerDomain.interval(0, 1).value!!,
                    scope = "stable",
                    origin = "legacy/x"
                ).ok
            )
            val currentSnapshotJson = ConstraintProgrammingSnapshotCodec.encode(model.snapshot().value!!).value!!
            val root = Json.parseToJsonElement(currentSnapshotJson).jsonObject.toMutableMap()
            root["variables"] = JsonArray(root.getValue("variables").jsonArray.map { element ->
                JsonObject(element.jsonObject.toMutableMap().apply {
                    remove("identityProvenance")
                    this["scope"] = JsonPrimitive("STABLE")
                })
            })
            val historicalSnapshotJson = Json.encodeToString(JsonObject(root))
            val checkpoint = PortableCheckpointCodec.decodeOrNull(
                PortableCheckpointCodec.encode(
                    PortableCheckpointEnvelope(
                        checkpointId = "historical-v2",
                        identitySchemaVersion = "1.0",
                        identityNamespace = "historical-fixture",
                        modelName = "historical-checkpoint-validation",
                        modelFingerprint = fingerprint(historicalSnapshotJson),
                        createdAtEpochMs = 1L,
                        snapshotJson = historicalSnapshotJson
                    )
                )
            )

            assertNotNull(checkpoint)
            assertTrue(
                OspfCpSnapshotExecutor(RecordingStorage()).validateExternalCheckpoint(
                    snapshotJson = currentSnapshotJson,
                    checkpoint = checkpoint
                )
            )
        } finally {
            model.close()
        }
    }

    /**
     * Verify the full calculator execution path restores a historical V2 checkpoint after snapshot
     * canonicalization. / 验证 calculator 完整执行路径会在 snapshot 规范化后恢复历史 V2 checkpoint。
     */
    @Test
    fun executeRestoresHistoricalV2CheckpointAfterCanonicalization() = runBlocking {
        assumeScipNativeAvailable()
        val model = ConstraintProgrammingModel(
            name = "historical-checkpoint-execute",
            identityNamespace = "historical-execute",
            identitySchemaVersion = "1.0"
        )
        val variable = fuookami.ospf.kotlin.core.variable.IntVar("x")
        try {
            assertTrue(
                model.registerVariable(
                    id = VariableId("historical:variable:x"),
                    variable = variable,
                    domain = IntegerDomain.interval(0, 1).value!!,
                    scope = "stable",
                    origin = "legacy/x"
                ).ok
            )
            val currentSnapshotJson = ConstraintProgrammingSnapshotCodec.encode(model.snapshot().value!!).value!!
            val root = Json.parseToJsonElement(currentSnapshotJson).jsonObject.toMutableMap()
            root["variables"] = JsonArray(root.getValue("variables").jsonArray.map { element ->
                JsonObject(element.jsonObject.toMutableMap().apply {
                    remove("identityProvenance")
                    this["scope"] = JsonPrimitive("STABLE")
                })
            })
            val historicalSnapshotJson = Json.encodeToString(JsonObject(root))
            val variableId = model.snapshot().value!!.variables.single().id.value
            val checkpoint = PortableCheckpointCodec.decodeOrNull(
                PortableCheckpointCodec.encode(
                    PortableCheckpointEnvelope(
                        checkpointId = "historical-execute",
                        identitySchemaVersion = "1.0",
                        identityNamespace = "historical-execute",
                        modelName = "historical-checkpoint-execute",
                        modelFingerprint = fingerprint(historicalSnapshotJson),
                        configurationFingerprint = fingerprint(configurationCanonical()),
                        solverFingerprint = scipRuntimeFingerprint(),
                        runId = "task",
                        attemptId = "slice",
                        createdAtEpochMs = 1L,
                        snapshotJson = historicalSnapshotJson,
                        incumbent = fuookami.ospf.framework.remote_solver.protocol.domain.PortableConstraintProgrammingIncumbent(
                            valuesById = mapOf(variableId to 0L)
                        )
                    )
                )
            )
            assertNotNull(checkpoint)

            val result = OspfCpSnapshotExecutor(RecordingStorage()).execute(
                payload = SolvePayload(
                    modelData = ModelData.raw(
                        bytes = currentSnapshotJson.encodeToByteArray(),
                        format = "ospf-cp-snapshot-json"
                    )
                ),
                tenantId = "tenant",
                taskId = "task",
                sliceId = "slice",
                checkpoint = checkpoint
            )

            assertNotEquals(RemoteTerminationReason.BACKEND_FAILURE, result.terminationReason, result.message ?: "")
            assertEquals("task", result.runId)
            assertEquals("slice", result.attemptId)
        } finally {
            model.close()
        }
    }

    /**
     * Skip native execution when the separately provided SCIP runtime is unavailable. /
     * 未提供独立 SCIP native runtime 时跳过 native 执行测试。
     */
    private fun assumeScipNativeAvailable() {
        val available = runCatching {
            val scipClass = Class.forName("jscip.Scip")
            val scip = scipClass.getDeclaredConstructor().newInstance()
            try {
                scipClass.getMethod("create", String::class.java)
                    .invoke(scip, "cp3-native-probe")
                true
            } finally {
                scipClass.getMethod("free").invoke(scip)
            }
        }.getOrDefault(false)
        assumeTrue(
            available,
            "SCIP native runtime is unavailable; native calculator execution is an environment-bound test"
        )
    }

    @Test
    fun externalSolutionSemanticValidationRejectsConstraintViolation() {
        val model = ConstraintProgrammingModel("external-invalid-solution")
        try {
            val variable = fuookami.ospf.kotlin.core.variable.IntVar("x")
            assertTrue(model.registerVariable(variable, IntegerDomain.interval(0, 1).value!!).ok)
            val expression = ConstraintProgrammingExpression.Variable(variable)
            assertTrue(
                model.addConstraint(
                    constraint = ConstraintProgrammingConstraint.equal(expression, Int64.zero).value!!,
                    id = "external-invalid-constraint"
                ).ok
            )
            val snapshotJson = ConstraintProgrammingSnapshotCodec.encode(model.snapshot().value!!).value!!
            val variableId = model.snapshot().value!!.variables.single().id.value
            val solution = SerializedSolution(
                feasible = true,
                optimal = false,
                variableValuesById = mapOf(variableId to 1L),
                problemStatus = RemoteProblemStatus.FEASIBLE,
                solutionPresence = RemoteSolutionPresence.INCUMBENT,
                proofStatus = RemoteProofStatus.NONE,
                terminationReason = RemoteTerminationReason.COMPLETED
            )

            assertFalse(OspfCpSnapshotExecutor(RecordingStorage()).validateExternalSolution(snapshotJson, solution))
        } finally {
            model.close()
        }
    }

    @Test
    fun externalSolutionSemanticValidationRejectsCompatibilityObjectiveAndPresenceMismatch() {
        val snapshotJson = snapshotJson()
        val solution = SerializedSolution(
            feasible = true,
            optimal = true,
            objectiveValue = Flt64.one,
            problemStatus = RemoteProblemStatus.FEASIBLE,
            solutionPresence = RemoteSolutionPresence.INCUMBENT,
            proofStatus = RemoteProofStatus.NONE,
            terminationReason = RemoteTerminationReason.COMPLETED
        )

        assertFalse(OspfCpSnapshotExecutor(RecordingStorage()).validateExternalSolution(snapshotJson, solution))
    }

    @Test
    fun optimalSolutionMayBeVerifiedWithoutInfeasibilityDiagnostics() {
        val (snapshotJson, variableId) = objectiveSnapshotJson()
        val solution = SerializedSolution(
            feasible = true,
            optimal = true,
            objectiveValueInt64 = 0L,
            variableValuesById = mapOf(variableId to 0L),
            problemStatus = RemoteProblemStatus.FEASIBLE,
            solutionPresence = RemoteSolutionPresence.OPTIMAL,
            proofStatus = RemoteProofStatus.VERIFIED,
            terminationReason = RemoteTerminationReason.COMPLETED
        )

        assertTrue(OspfCpSnapshotExecutor(RecordingStorage()).validateExternalSolution(snapshotJson, solution))
    }

    @Test
    fun backendFailureIncumbentIsValidatedTheSameWithOrWithoutAnObjective() {
        val (snapshotJson, variableId) = objectiveSnapshotJson()
        val solution = SerializedSolution(
            feasible = true,
            optimal = false,
            variableValuesById = mapOf(variableId to 0L),
            problemStatus = RemoteProblemStatus.FEASIBLE,
            solutionPresence = RemoteSolutionPresence.INCUMBENT,
            proofStatus = RemoteProofStatus.NONE,
            terminationReason = RemoteTerminationReason.BACKEND_FAILURE
        )
        assertTrue(OspfCpSnapshotExecutor(RecordingStorage()).validateExternalSolution(snapshotJson, solution))
        assertTrue(
            OspfCpSnapshotExecutor(RecordingStorage()).validateExternalSolution(
                snapshotJson(),
                solution.copy(variableValuesById = emptyMap())
            )
        )
    }

    /**
     * 可行 incumbent 不得同时携带不可行冲突证据。
     * A feasible incumbent must not carry infeasibility conflict evidence.
     */
    @Test
    fun feasibleSolutionRejectsInfeasibilityEvidence() {
        val (snapshotJson, variableId) = objectiveSnapshotJson()
        val diagnostics = RecordingStorage().json
        val solution = SerializedSolution(
            feasible = true,
            optimal = false,
            variableValuesById = mapOf(variableId to 0L),
            problemStatus = RemoteProblemStatus.FEASIBLE,
            solutionPresence = RemoteSolutionPresence.INCUMBENT,
            proofStatus = RemoteProofStatus.NONE,
            terminationReason = RemoteTerminationReason.COMPLETED,
            diagnostics = mapOf(
                "infeasibility.validity" to "Verified",
                "infeasibility.minimality" to "Irreducible",
                "infeasibility.members" to diagnostics.encodeToString(listOf("constraint:unexpected"))
            )
        )

        assertFalse(OspfCpSnapshotExecutor(RecordingStorage()).validateExternalSolution(snapshotJson, solution))
    }

    @Test
    fun nonBackendFailureIncumbentStillRequiresAnExactObjective() {
        val (snapshotJson, variableId) = objectiveSnapshotJson()
        val solution = SerializedSolution(
            feasible = true,
            optimal = false,
            variableValuesById = mapOf(variableId to 0L),
            problemStatus = RemoteProblemStatus.FEASIBLE,
            solutionPresence = RemoteSolutionPresence.INCUMBENT,
            proofStatus = RemoteProofStatus.NONE,
            terminationReason = RemoteTerminationReason.COMPLETED
        )
        assertFalse(OspfCpSnapshotExecutor(RecordingStorage()).validateExternalSolution(snapshotJson, solution))
    }

    @Test
    fun backendFailureCheckpointExportRecomputesMissingObjective() = runBlocking {
        val storage = RecordingStorage()
        val (snapshot, variableId) = objectiveSnapshotJson()
        val modelFingerprint = fingerprint(snapshot)
        val configurationFingerprint = fingerprint(configurationCanonical())
        val solverFingerprint = scipRuntimeFingerprint()
        val artifact = SerializedSolution(
            feasible = true,
            optimal = false,
            variableValuesById = mapOf(variableId to 0L),
            problemStatus = RemoteProblemStatus.FEASIBLE,
            solutionPresence = RemoteSolutionPresence.INCUMBENT,
            proofStatus = RemoteProofStatus.NONE,
            terminationReason = RemoteTerminationReason.BACKEND_FAILURE,
            schemaVersion = "2.0",
            runId = "task",
            attemptId = "slice",
            fingerprints = mapOf(
                "model" to modelFingerprint,
                "configuration" to configurationFingerprint,
                "solver" to solverFingerprint
            ),
            fingerprintSchemas = mapOf(
                "model" to "1.0",
                "configuration" to "1.0",
                "solver" to "2.0"
            )
        )
        val digest = digest(artifact)
        val resultRef = storage.put(
            path = "tenant/result/task/slice",
            bytes = storage.json.encodeToString(
                SerializedSolution.serializer(),
                artifact.copy(artifactDigest = digest)
            ).encodeToByteArray()
        )
        val result = SolveResult(
            feasible = true,
            optimal = false,
            objectiveValue = null,
            objectiveValueInt64 = null,
            gap = null,
            elapsed = Duration.ZERO,
            resultRef = resultRef,
            schemaVersion = "2.0",
            problemStatus = RemoteProblemStatus.FEASIBLE,
            solutionPresence = RemoteSolutionPresence.INCUMBENT,
            proofStatus = RemoteProofStatus.NONE,
            terminationReason = RemoteTerminationReason.BACKEND_FAILURE,
            fingerprints = artifact.fingerprints,
            fingerprintSchemas = artifact.fingerprintSchemas,
            runId = "task",
            attemptId = "slice",
            artifactDigest = digest
        )

        val checkpointRef = OspfCpSnapshotExecutor(storage).exportCheckpoint(
            payload = SolvePayload(
                modelData = ModelData.raw(snapshot.encodeToByteArray(), "ospf-cp-snapshot-json")
            ),
            result = result,
            tenantId = "tenant",
            taskId = "task",
            sliceId = "slice"
        )
        assertNotNull(checkpointRef)
        val checkpoint = PortableCheckpointCodec.decodeOrNull(storage.get(checkpointRef)!!.decodeToString())
        assertEquals("0", checkpoint?.incumbent?.objective)
    }

    @Test
    fun externalDiagnosticsRejectUnknownFields() {
        val model = ConstraintProgrammingModel("diagnostics")
        try {
            val snapshot = model.snapshot().value!!
            assertFalse(
                OspfCpSnapshotExecutor(RecordingStorage()).validateExternalDiagnostics(
                    snapshot,
                    mapOf("infeasibility.unknown" to "value")
                )
            )
        } finally {
            model.close()
        }
    }

    @Test
    fun externalDiagnosticsAcceptValidMembersAndRejectUnknownMembers() {
        val model = ConstraintProgrammingModel("diagnostics-members")
        try {
            val variable = fuookami.ospf.kotlin.core.variable.IntVar("x")
            assertTrue(model.registerVariable(variable, IntegerDomain.interval(0, 1).value!!).ok)
            val expression = ConstraintProgrammingExpression.Variable(variable)
            val constraintId = "diagnostics-members-constraint"
            assertTrue(
                model.addConstraint(
                    constraint = ConstraintProgrammingConstraint.equal(expression, Int64.zero).value!!,
                    id = constraintId
                ).ok
            )
            val snapshot = model.snapshot().value!!
            val variableId = snapshot.variables.single().id.value
            val valid = mapOf(
                "infeasibility.source" to "ConstraintConflict",
                "infeasibility.validity" to "Verified",
                "infeasibility.minimality" to "Irreducible",
                "infeasibility.constraintIds" to "[\"$constraintId\"]",
                "infeasibility.variableDomainRefs" to "[\"$variableId\"]",
                "infeasibility.members" to "[\"constraint:$constraintId\",\"domain:$variableId\"]"
            )
            val executor = OspfCpSnapshotExecutor(RecordingStorage())
            assertTrue(executor.validateExternalDiagnostics(snapshot, valid))
            assertFalse(
                executor.validateExternalDiagnostics(
                    snapshot,
                    valid + ("infeasibility.members" to "[\"constraint:unknown\"]")
                )
            )
            assertFalse(
                executor.validateExternalDiagnostics(
                    snapshot,
                    valid + ("infeasibility.variableDomainRefs" to "[]")
                )
            )
        } finally {
            model.close()
        }
    }

    @Test
    fun externalCheckpointSemanticValidationRejectsInvalidIncumbent() {
        val model = ConstraintProgrammingModel("external-invalid-checkpoint")
        try {
            val variable = fuookami.ospf.kotlin.core.variable.IntVar("x")
            assertTrue(model.registerVariable(variable, IntegerDomain.interval(0, 1).value!!).ok)
            val expression = ConstraintProgrammingExpression.Variable(variable)
            assertTrue(
                model.addConstraint(
                    constraint = ConstraintProgrammingConstraint.equal(expression, Int64.zero).value!!,
                    id = "external-invalid-checkpoint-constraint"
                ).ok
            )
            val snapshotJson = ConstraintProgrammingSnapshotCodec.encode(model.snapshot().value!!).value!!
            val variableId = model.snapshot().value!!.variables.single().id.value
            val checkpoint = PortableCheckpointEnvelope(
                checkpointId = "historical-slice",
                modelFingerprint = fingerprint(snapshotJson),
                configurationFingerprint = fingerprint(configurationCanonical()),
                solverFingerprint = scipRuntimeFingerprint(),
                runId = "task",
                attemptId = "previous-slice",
                createdAtEpochMs = 1L,
                snapshotJson = snapshotJson,
                incumbent = fuookami.ospf.framework.remote_solver.protocol.domain.PortableConstraintProgrammingIncumbent(
                    valuesById = mapOf(variableId to 1L)
                )
            )

            assertFalse(OspfCpSnapshotExecutor(RecordingStorage()).validateExternalCheckpoint(snapshotJson, checkpoint))
        } finally {
            model.close()
        }
    }

    /**
     * Plain CP must reject Benders checkpoint state instead of silently dropping it.
     * 普通 CP 必须拒绝 Benders checkpoint 状态，不得静默丢弃。
     */
    @Test
    fun externalCheckpointRejectsBendersStateForPlainCpExecutor() {
        val snapshotJson = snapshotJson()
        val checkpoint = PortableCheckpointEnvelope(
            checkpointId = "benders-input",
            modelFingerprint = fingerprint(snapshotJson),
            createdAtEpochMs = 1L,
            snapshotJson = snapshotJson,
            benders = PortableConstraintProgrammingBendersState(
                iteration = 1L,
                masterFingerprint = "master",
                cuts = emptyList()
            )
        )

        assertFalse(OspfCpSnapshotExecutor(RecordingStorage()).validateExternalCheckpoint(snapshotJson, checkpoint))
    }

    /**
     * Contradictory source and verified evidence members must be rejected.
     * source 与 Verified 成员互相矛盾时必须拒绝。
     */
    @Test
    fun externalDiagnosticsRejectsNoneSourceWithVerifiedMembers() {
        val model = ConstraintProgrammingModel("diagnostics-none-source")
        try {
            val snapshot = model.snapshot().value!!
            val diagnostics = mapOf(
                "infeasibility.source" to "None",
                "infeasibility.validity" to "Verified",
                "infeasibility.minimality" to "Irreducible",
                "infeasibility.members" to "[\"constraint:unexpected\"]"
            )
            assertFalse(OspfCpSnapshotExecutor(RecordingStorage()).validateExternalDiagnostics(snapshot, diagnostics))
        } finally {
            model.close()
        }
    }

    @Test
    fun cpObjectiveProducerRawAndArtifactKeepExactInt64() {
        val json = Json { encodeDefaults = true }
        listOf(9_007_199_254_740_993L, Long.MAX_VALUE, Long.MIN_VALUE).forEach { value ->
            val fields = exactCpObjectiveFields(Int64(value))
            assertEquals(null, fields.objectiveValue)
            assertEquals(value, fields.objectiveValueInt64)

            val artifact = SerializedSolution(
                feasible = true,
                optimal = false,
                objectiveValue = fields.objectiveValue,
                objectiveValueInt64 = fields.objectiveValueInt64
            )
            val decodedArtifact = json.decodeFromString(
                SerializedSolution.serializer(),
                json.encodeToString(
                    SerializedSolution.serializer(),
                    artifact
                )
            )
            assertEquals(null, decodedArtifact.objectiveValue)
            assertEquals(value, decodedArtifact.objectiveValueInt64)

            val raw = SolveResult(
                feasible = true,
                optimal = false,
                objectiveValue = fields.objectiveValue,
                objectiveValueInt64 = fields.objectiveValueInt64,
                gap = null,
                elapsed = Duration.ZERO
            )
            assertEquals(null, raw.objectiveValue)
            assertEquals(value, raw.objectiveValueInt64)
        }
    }

    @Test
    fun conflictingFeasibleArtifactDoesNotBecomeCheckpointIncumbent() = runBlocking {
        val storage = RecordingStorage()
        val snapshotJson = snapshotJson()
        val modelFingerprint = fingerprint(snapshotJson)
        val configurationFingerprint = fingerprint(configurationCanonical())
        val solverFingerprint = fingerprint("scip-cp")
        val solution = SerializedSolution(
            feasible = true,
            optimal = false,
            problemStatus = RemoteProblemStatus.FEASIBLE,
            solutionPresence = RemoteSolutionPresence.INCUMBENT,
            terminationReason = RemoteTerminationReason.COMPLETED,
            fingerprints = mapOf(
                "model" to modelFingerprint,
                "configuration" to configurationFingerprint,
                "solver" to solverFingerprint
            ),
            runId = "task",
            attemptId = "slice"
        )
        val digest = digest(solution)
        val resultRef = storage.put(
            path = "tenant/result/task/slice",
            bytes = storage.json.encodeToString(
                SerializedSolution.serializer(),
                solution.copy(artifactDigest = digest)
            ).encodeToByteArray()
        )
        val payload = SolvePayload(
            modelData = ModelData.raw(
                bytes = snapshotJson.encodeToByteArray(),
                format = "ospf-cp-snapshot-json"
            )
        )
        val result = SolveResult(
            feasible = false,
            optimal = false,
            objectiveValue = null,
            gap = null,
            elapsed = Duration.ZERO,
            problemStatus = RemoteProblemStatus.INFEASIBLE,
            terminationReason = RemoteTerminationReason.TIME_LIMIT,
            solutionPresence = RemoteSolutionPresence.NONE,
            fingerprints = solution.fingerprints,
            runId = "task",
            attemptId = "slice",
            resultRef = resultRef,
            artifactDigest = digest
        )
        val checkpointRef = OspfCpSnapshotExecutor(storage).exportCheckpoint(
            payload = payload,
            result = result,
            tenantId = "tenant",
            taskId = "task",
            sliceId = "slice"
        )
        assertNull(checkpointRef)
    }

    @Test
    fun checkpointPreservesAssumptionsAndConflictMembers() = runBlocking {
        val storage = RecordingStorage()
        val model = ConstraintProgrammingModel("checkpoint-evidence")
        val variable = fuookami.ospf.kotlin.core.variable.IntVar("checkpoint-evidence-x")
        model.registerVariable(variable, IntegerDomain.interval(0, 1).value!!)
        val expression = ConstraintProgrammingExpression.Variable(variable)
        model.addConstraint(
            constraint = ConstraintProgrammingConstraint.equal(expression, Int64.zero).value!!,
            id = "evidence:constraint"
        )
        val snapshot = model.snapshot().value!!
        val variableId = snapshot.variables.single().id.value
        val snapshotJson = try {
            ConstraintProgrammingSnapshotCodec.encode(snapshot).value!!
        } finally {
            model.close()
        }
        val diagnostics = mapOf(
            "infeasibility.source" to "ConstraintConflict",
            "infeasibility.assumptionIds" to storage.json.encodeToString(listOf(variableId)),
            "infeasibility.constraintIds" to storage.json.encodeToString(listOf("evidence:constraint")),
            "infeasibility.variableBoundRefs" to storage.json.encodeToString(emptyList<String>()),
            "infeasibility.variableDomainRefs" to storage.json.encodeToString(emptyList<String>()),
            "infeasibility.members" to storage.json.encodeToString(listOf("constraint:evidence:constraint")),
            "infeasibility.validity" to "Verified",
            "infeasibility.minimality" to "Irreducible"
        )
        val fingerprints = mapOf(
            "model" to fingerprint(snapshotJson),
            "configuration" to fingerprint(configurationCanonical()),
            "solver" to scipRuntimeFingerprint()
        )
        val fingerprintSchemas = mapOf(
            "model" to "1.0",
            "configuration" to "1.0",
            "solver" to "2.0"
        )
        val artifact = SerializedSolution(
            feasible = false,
            optimal = false,
            problemStatus = RemoteProblemStatus.INFEASIBLE,
            solutionPresence = RemoteSolutionPresence.NONE,
            proofStatus = RemoteProofStatus.VERIFIED,
            terminationReason = RemoteTerminationReason.TIME_LIMIT,
            schemaVersion = "2.0",
            fingerprints = fingerprints,
            fingerprintSchemas = fingerprintSchemas,
            diagnostics = diagnostics,
            runId = "task",
            attemptId = "slice"
        )
        val artifactDigest = digest(artifact)
        val resultRef = storage.put(
            path = "tenant/result/task/slice",
            bytes = storage.json.encodeToString(
                SerializedSolution.serializer(),
                artifact.copy(artifactDigest = artifactDigest)
            ).encodeToByteArray()
        )
        val checkpointRef = OspfCpSnapshotExecutor(storage).exportCheckpoint(
            payload = SolvePayload(
                modelData = ModelData.raw(snapshotJson.encodeToByteArray(), "ospf-cp-snapshot-json")
            ),
            result = SolveResult(
                feasible = false,
                optimal = false,
                objectiveValue = null,
                gap = null,
                elapsed = Duration.ZERO,
                problemStatus = RemoteProblemStatus.INFEASIBLE,
                terminationReason = RemoteTerminationReason.TIME_LIMIT,
                solutionPresence = RemoteSolutionPresence.NONE,
                proofStatus = RemoteProofStatus.VERIFIED,
                schemaVersion = "2.0",
                fingerprints = fingerprints,
                fingerprintSchemas = fingerprintSchemas,
                diagnostics = diagnostics,
                runId = "task",
                attemptId = "slice",
                resultRef = resultRef,
                artifactDigest = artifactDigest
            ),
            tenantId = "tenant",
            taskId = "task",
            sliceId = "slice"
        )
        assertNotNull(checkpointRef)
        val checkpoint = PortableCheckpointCodec.decodeOrNull(
            storage.get(checkpointRef)!!.decodeToString()
        )
        assertNotNull(checkpoint)
        assertEquals(listOf(variableId), checkpoint.assumptions)
        assertEquals(listOf("constraint:evidence:constraint"), checkpoint.conflicts.single().memberIds)
    }

    @Test
    fun malformedDiagnosticListDoesNotProduceCheckpointEvidence() = runBlocking {
        val storage = RecordingStorage()
        val snapshotJson = snapshotJson()
        val checkpointRef = OspfCpSnapshotExecutor(storage).exportCheckpoint(
            payload = SolvePayload(
                modelData = ModelData.raw(snapshotJson.encodeToByteArray(), "ospf-cp-snapshot-json")
            ),
            result = SolveResult(
                feasible = false,
                optimal = false,
                objectiveValue = null,
                gap = null,
                elapsed = Duration.ZERO,
                problemStatus = RemoteProblemStatus.UNKNOWN,
                terminationReason = RemoteTerminationReason.TIME_LIMIT,
                solutionPresence = RemoteSolutionPresence.NONE,
                fingerprints = mapOf(
                    "model" to fingerprint(snapshotJson),
                    "configuration" to fingerprint(configurationCanonical()),
                    "solver" to scipRuntimeFingerprint()
                ),
                diagnostics = mapOf("infeasibility.members" to "not-json")
            ),
            tenantId = "tenant",
            taskId = "task",
            sliceId = "slice"
        )
        assertNull(checkpointRef)
    }

    @Test
    fun nonIncumbentArtifactCannotCarryAnObjective() = runBlocking {
        val storage = RecordingStorage()
        val snapshot = snapshotJson()
        val fingerprints = mapOf(
            "model" to fingerprint(snapshot),
            "configuration" to fingerprint(configurationCanonical()),
            "solver" to scipRuntimeFingerprint()
        )
        val artifact = SerializedSolution(
            feasible = false,
            optimal = false,
            objectiveValueInt64 = 7L,
            problemStatus = RemoteProblemStatus.UNKNOWN,
            solutionPresence = RemoteSolutionPresence.NONE,
            proofStatus = RemoteProofStatus.NONE,
            terminationReason = RemoteTerminationReason.TIME_LIMIT,
            schemaVersion = "2.0",
            fingerprints = fingerprints,
            fingerprintSchemas = mapOf("model" to "1.0", "configuration" to "1.0", "solver" to "2.0"),
            runId = "task",
            attemptId = "slice"
        )
        val artifactDigest = digest(artifact)
        val resultRef = storage.put(
            path = "tenant/result/task/slice",
            bytes = storage.json.encodeToString(
                SerializedSolution.serializer(),
                artifact.copy(artifactDigest = artifactDigest)
            ).encodeToByteArray()
        )

        val checkpoint = OspfCpSnapshotExecutor(storage).exportCheckpoint(
            payload = SolvePayload(ModelData.raw(snapshot.encodeToByteArray(), "ospf-cp-snapshot-json")),
            result = SolveResult(
                feasible = false,
                optimal = false,
                objectiveValue = null,
                gap = null,
                elapsed = Duration.ZERO,
                problemStatus = RemoteProblemStatus.UNKNOWN,
                terminationReason = RemoteTerminationReason.TIME_LIMIT,
                solutionPresence = RemoteSolutionPresence.NONE,
                proofStatus = RemoteProofStatus.NONE,
                schemaVersion = "2.0",
                fingerprints = fingerprints,
                fingerprintSchemas = artifact.fingerprintSchemas,
                runId = "task",
                attemptId = "slice",
                resultRef = resultRef,
                artifactDigest = artifactDigest
            ),
            tenantId = "tenant",
            taskId = "task",
            sliceId = "slice"
        )

        assertNull(checkpoint)
    }

    @Test
    fun checkpointRejectsFingerprintSchemaMismatchBetweenRawAndArtifact() = runBlocking {
        val storage = RecordingStorage()
        val snapshotJson = snapshotJson()
        val modelFingerprint = fingerprint(snapshotJson)
        val configurationFingerprint = fingerprint(configurationCanonical())
        val solverFingerprint = fingerprint("scip-cp")
        val artifactSchemas = mapOf(
            "model" to "1.0",
            "configuration" to "1.0",
            "solver" to "1.0"
        )
        val rawSchemas = artifactSchemas + ("solver" to "2.0")
        val solution = SerializedSolution(
            feasible = true,
            optimal = false,
            problemStatus = RemoteProblemStatus.FEASIBLE,
            solutionPresence = RemoteSolutionPresence.INCUMBENT,
            proofStatus = RemoteProofStatus.NONE,
            terminationReason = RemoteTerminationReason.COMPLETED,
            fingerprints = mapOf(
                "model" to modelFingerprint,
                "configuration" to configurationFingerprint,
                "solver" to solverFingerprint
            ),
            fingerprintSchemas = artifactSchemas,
            runId = "task",
            attemptId = "slice"
        )
        val digest = digest(solution)
        val resultRef = storage.put(
            path = "tenant/result/task/slice",
            bytes = storage.json.encodeToString(
                SerializedSolution.serializer(),
                solution.copy(artifactDigest = digest)
            ).encodeToByteArray()
        )
        val result = SolveResult(
            feasible = true,
            optimal = false,
            objectiveValue = null,
            gap = null,
            elapsed = Duration.ZERO,
            problemStatus = RemoteProblemStatus.FEASIBLE,
            terminationReason = RemoteTerminationReason.COMPLETED,
            solutionPresence = RemoteSolutionPresence.INCUMBENT,
            proofStatus = RemoteProofStatus.NONE,
            fingerprints = solution.fingerprints,
            fingerprintSchemas = rawSchemas,
            runId = "task",
            attemptId = "slice",
            resultRef = resultRef,
            artifactDigest = digest
        )
        val checkpointRef = OspfCpSnapshotExecutor(storage).exportCheckpoint(
            payload = SolvePayload(
                modelData = ModelData.raw(snapshotJson.encodeToByteArray(), "ospf-cp-snapshot-json")
            ),
            result = result,
            tenantId = "tenant",
            taskId = "task",
            sliceId = "slice"
        )
        assertNull(checkpointRef)
    }

    @Test
    fun v2CheckpointCannotMasqueradeAsLegacyV1() = runBlocking {
        val storage = RecordingStorage()
        val snapshotJson = snapshotJson()
        val envelope = PortableCheckpointEnvelope(
            schemaVersion = "2.0",
            sourceFormat = "v2",
            checkpointId = "legacy-v1",
            modelName = "checkpoint-model",
            modelFingerprint = fingerprint(snapshotJson),
            configurationFingerprint = null,
            solverFingerprint = null,
            createdAtEpochMs = 0L,
            snapshotJson = snapshotJson
        )
        val decoded = PortableCheckpointCodec.decodeOrNull(
            PortableCheckpointCodec.encode(envelope)
        )
        assertNull(decoded)

        val result = OspfCpSnapshotExecutor(storage).execute(
            payload = SolvePayload(
                modelData = ModelData.raw(
                    bytes = snapshotJson.encodeToByteArray(),
                    format = "ospf-cp-snapshot-json"
                )
            ),
            tenantId = "tenant",
            taskId = "task",
            sliceId = "slice",
            checkpoint = envelope.copy(
                integritySha256 = fingerprint(
                    Json { encodeDefaults = true }.encodeToString(
                        PortableCheckpointEnvelope.serializer(),
                        envelope.copy(integritySha256 = "")
                    )
                )
            )
        )

        assertEquals(RemoteTerminationReason.BACKEND_FAILURE, result.terminationReason)
        assertEquals(RemoteProblemStatus.UNKNOWN, result.problemStatus)
        assertEquals(RemoteSolutionPresence.NONE, result.solutionPresence)
    }

    @Test
    fun executeRejectsTypedV2CheckpointWithInvalidIntegrity() = runBlocking {
        val storage = RecordingStorage()
        val snapshotJson = snapshotJson()
        val checkpoint = PortableCheckpointEnvelope(
            checkpointId = "corrupt-integrity",
            identitySchemaVersion = "1.0",
            identityNamespace = "model-local",
            modelName = "checkpoint-model",
            modelFingerprint = fingerprint(snapshotJson),
            configurationFingerprint = fingerprint(configurationCanonical()),
            solverFingerprint = scipRuntimeFingerprint(),
            runId = "task",
            attemptId = "slice",
            createdAtEpochMs = 1L,
            snapshotJson = snapshotJson,
            integritySha256 = "corrupt"
        )
        val result = OspfCpSnapshotExecutor(storage).execute(
            payload = SolvePayload(
                modelData = ModelData.raw(snapshotJson.encodeToByteArray(), "ospf-cp-snapshot-json")
            ),
            tenantId = "tenant",
            taskId = "task",
            sliceId = "slice",
            checkpoint = checkpoint
        )

        assertEquals(RemoteTerminationReason.BACKEND_FAILURE, result.terminationReason)
        assertEquals(RemoteProblemStatus.UNKNOWN, result.problemStatus)
        assertEquals(RemoteSolutionPresence.NONE, result.solutionPresence)
    }

    @Test
    fun unsupportedSolverParamsFailBeforeSnapshotExecution() = runBlocking {
        val payload = SolvePayload(
            modelData = ModelData.raw(
                bytes = ByteArray(0),
                format = "ospf-cp-snapshot-json"
            ),
            config = SolverConfig(timeLimit = null, solverParams = mapOf("presolve" to "aggressive"))
        )
        val result = OspfCpSnapshotExecutor(RecordingStorage()).execute(
            payload = payload,
            tenantId = "tenant",
            taskId = "task",
            sliceId = "slice"
        )
        assertEquals(RemoteTerminationReason.BACKEND_FAILURE, result.terminationReason)
        assertEquals(RemoteProblemStatus.UNKNOWN, result.problemStatus)
        assertEquals("2.0", result.schemaVersion)
        assertEquals("task", result.runId)
        assertEquals("slice", result.attemptId)
    }

    @Test
    fun checkpointAcceptsArtifactElapsedAtProtocolMillisecondPrecision() {
        runBlocking {
            val storage = RecordingStorage()
            val snapshotJson = snapshotJson()
            val elapsed = 1_500_000L.nanoseconds
            val modelFingerprint = fingerprint(snapshotJson)
            val configurationFingerprint = fingerprint(configurationCanonical())
            val solverFingerprint = fingerprint("scip-cp")
            val solution = SerializedSolution(
                feasible = true,
                optimal = false,
                elapsed = elapsed,
                problemStatus = RemoteProblemStatus.FEASIBLE,
                solutionPresence = RemoteSolutionPresence.INCUMBENT,
                proofStatus = RemoteProofStatus.NONE,
                terminationReason = RemoteTerminationReason.COMPLETED,
                fingerprints = mapOf(
                    "model" to modelFingerprint,
                    "configuration" to configurationFingerprint,
                    "solver" to solverFingerprint
                ),
                fingerprintSchemas = mapOf(
                    "model" to "1.0",
                    "configuration" to "1.0",
                    "solver" to "1.0"
                ),
                statistics = emptyMap(),
                diagnostics = emptyMap(),
                schemaVersion = "2.0",
                runId = "task",
                attemptId = "slice"
            )
            val digest = digest(solution)
            val ref = storage.put(
                path = "tenant/result/task/slice",
                bytes = storage.json.encodeToString(
                    SerializedSolution.serializer(),
                    solution.copy(artifactDigest = digest)
                ).encodeToByteArray()
            )
            val result = SolveResult(
                feasible = true,
                optimal = false,
                objectiveValue = null,
                gap = null,
                elapsed = elapsed,
                problemStatus = RemoteProblemStatus.FEASIBLE,
                terminationReason = RemoteTerminationReason.COMPLETED,
                solutionPresence = RemoteSolutionPresence.INCUMBENT,
                schemaVersion = "2.0",
                fingerprints = solution.fingerprints,
                fingerprintSchemas = solution.fingerprintSchemas,
                statistics = solution.statistics,
                diagnostics = solution.diagnostics,
                runId = "task",
                attemptId = "slice",
                resultRef = ref,
                artifactDigest = digest
            )
            val checkpoint = OspfCpSnapshotExecutor(storage).exportCheckpoint(
                payload = SolvePayload(
                    modelData = ModelData.raw(snapshotJson.encodeToByteArray(), "ospf-cp-snapshot-json")
                ),
                result = result,
                tenantId = "tenant",
                taskId = "task",
                sliceId = "slice"
            )
            kotlin.test.assertNotNull(checkpoint)
        }
    }

    @Test
    fun checkpointConfigurationFingerprintUsesEffectiveTaskMetadata() = runBlocking {
        val storage = RecordingStorage()
        val snapshot = snapshotJson()
        suspend fun resultFor(timeLimitMs: Long, sliceId: String): SolveResult {
            val fingerprints = mapOf(
                "model" to fingerprint(snapshot),
                "configuration" to fingerprint(configurationCanonical(timeLimitMs.toString())),
                "solver" to scipRuntimeFingerprint()
            )
            val fingerprintSchemas = mapOf(
                "model" to "1.0",
                "configuration" to "1.0",
                "solver" to "2.0"
            )
            val artifact = SerializedSolution(
                feasible = false,
                optimal = false,
                problemStatus = RemoteProblemStatus.UNKNOWN,
                terminationReason = RemoteTerminationReason.TIME_LIMIT,
                solutionPresence = RemoteSolutionPresence.NONE,
                proofStatus = RemoteProofStatus.NONE,
                schemaVersion = "2.0",
                fingerprints = fingerprints,
                fingerprintSchemas = fingerprintSchemas,
                runId = "task",
                attemptId = sliceId
            )
            val artifactDigest = digest(artifact)
            val resultRef = storage.put(
                path = "tenant/result/task/$sliceId",
                bytes = storage.json.encodeToString(
                    SerializedSolution.serializer(),
                    artifact.copy(artifactDigest = artifactDigest)
                ).encodeToByteArray()
            )
            return SolveResult(
                feasible = false,
                optimal = false,
                objectiveValue = null,
                gap = null,
                elapsed = Duration.ZERO,
                problemStatus = RemoteProblemStatus.UNKNOWN,
                terminationReason = RemoteTerminationReason.TIME_LIMIT,
                solutionPresence = RemoteSolutionPresence.NONE,
                schemaVersion = "2.0",
                fingerprints = fingerprints,
                fingerprintSchemas = fingerprintSchemas,
                runId = "task",
                attemptId = sliceId,
                resultRef = resultRef,
                artifactDigest = artifactDigest
            )
        }
        val first = OspfCpSnapshotExecutor(storage).exportCheckpoint(
            payload = SolvePayload(
                modelData = ModelData.raw(snapshot.encodeToByteArray(), "ospf-cp-snapshot-json"),
                taskMeta = TaskMeta(timeLimitMs = 1_000L)
            ),
            result = resultFor(1_000L, "first"),
            tenantId = "tenant",
            taskId = "task",
            sliceId = "first"
        )
        val second = OspfCpSnapshotExecutor(storage).exportCheckpoint(
            payload = SolvePayload(
                modelData = ModelData.raw(snapshot.encodeToByteArray(), "ospf-cp-snapshot-json"),
                taskMeta = TaskMeta(timeLimitMs = 2_000L)
            ),
            result = resultFor(2_000L, "second"),
            tenantId = "tenant",
            taskId = "task",
            sliceId = "second"
        )
        val firstCheckpoint = PortableCheckpointCodec.decodeOrNull(
            storage.get(first!!)!!.decodeToString()
        )!!
        val secondCheckpoint = PortableCheckpointCodec.decodeOrNull(
            storage.get(second!!)!!.decodeToString()
        )!!
        assertNotEquals(
            firstCheckpoint.configurationFingerprint,
            secondCheckpoint.configurationFingerprint
        )
    }

    @Test
    fun recognizesAllPublishedLegacyV2ConfigurationFingerprints() {
        val payload = SolvePayload(
            modelData = ModelData.raw(ByteArray(0), "ospf-cp-snapshot-json"),
            config = SolverConfig(
                timeLimitMs = null,
                solutionLimit = null,
                threads = null,
                solverParams = mapOf("presolve" to "on")
            ),
            taskMeta = TaskMeta(timeLimitMs = 1_500L, solutionLimit = 5)
        )
        val executor = OspfCpSnapshotExecutor(RecordingStorage())
        val fingerprints = executor.legacyV2ConfigurationFingerprints(payload)
        val raw = payload.config!!
        val delimiterFingerprint = fingerprint(legacyDelimiterCanonical(raw))
        val rawStructuredFingerprint = fingerprint(configurationCanonical(raw))
        val effective = raw.copy(
            timeLimit = payload.config?.timeLimit ?: payload.taskMeta.timeLimit,
            solutionLimit = payload.config?.solutionLimit ?: payload.taskMeta.solutionLimit
        )
        val metadataFallbackFingerprint = fingerprint(configurationCanonical(effective))
        assertEquals(3, fingerprints.size)
        assertTrue(fingerprints.contains(delimiterFingerprint))
        assertTrue(fingerprints.contains(rawStructuredFingerprint))
        assertTrue(fingerprints.contains(metadataFallbackFingerprint))
        assertNotEquals(delimiterFingerprint, rawStructuredFingerprint)
        assertNotEquals(rawStructuredFingerprint, metadataFallbackFingerprint)
        assertNotEquals(delimiterFingerprint, metadataFallbackFingerprint)
        assertTrue(executor.isLegacyV2ConfigurationFingerprint(payload, delimiterFingerprint))
        assertTrue(executor.isLegacyV2ConfigurationFingerprint(payload, rawStructuredFingerprint))
        assertTrue(executor.isLegacyV2ConfigurationFingerprint(payload, metadataFallbackFingerprint))
    }

    @Test
    fun rejectsUnknownV2SolverFingerprint() = runBlocking {
        val storage = RecordingStorage()
        val snapshot = snapshotJson()
        val result = SolveResult(
            feasible = false,
            optimal = false,
            objectiveValue = null,
            gap = null,
            elapsed = Duration.ZERO,
            problemStatus = RemoteProblemStatus.UNKNOWN,
            terminationReason = RemoteTerminationReason.TIME_LIMIT,
            solutionPresence = RemoteSolutionPresence.NONE,
            fingerprints = mapOf(
                "model" to fingerprint(snapshot),
                "configuration" to fingerprint(configurationCanonical(SolverConfig(timeLimit = null, threads = 8))),
                "solver" to "unknown-backend-fingerprint"
            )
        )
        val checkpoint = OspfCpSnapshotExecutor(storage).exportCheckpoint(
            payload = SolvePayload(
                modelData = ModelData.raw(snapshot.encodeToByteArray(), "ospf-cp-snapshot-json"),
                config = SolverConfig(timeLimit = null, threads = 8)
            ),
            result = result,
            tenantId = "tenant",
            taskId = "task",
            sliceId = "slice"
        )
        assertNull(checkpoint)
    }

    @Test
    fun acceptsOnlyExplicitLegacySolverFingerprints() {
        val executor = OspfCpSnapshotExecutor(RecordingStorage())
        val legacyRuntimeFingerprints = historicalRuntimeFingerprints()

        assertTrue(executor.isLegacyV2SolverFingerprint(fingerprint("scip-cp")))
        assertTrue(legacyRuntimeFingerprints.isNotEmpty())
        assertTrue(legacyRuntimeFingerprints.all(executor::isLegacyV2SolverFingerprint))
        assertFalse(executor.isLegacyV2SolverFingerprint("unknown-backend-fingerprint"))
        assertFalse(executor.isLegacyV2SolverFingerprint("scip-runtime-1:${"0".repeat(64)}"))
    }

    @Test
    fun checkpointAcceptsLegacyRuntimeFingerprintInSameEnvironment() {
        runBlocking {
            val storage = RecordingStorage()
            val snapshot = snapshotJson()
            historicalRuntimeFingerprints().forEachIndexed { index, legacyRuntimeFingerprint ->
                val sliceId = "legacy-runtime-$index"
                val fingerprints = mapOf(
                    "model" to fingerprint(snapshot),
                    "configuration" to fingerprint(configurationCanonical()),
                    "solver" to legacyRuntimeFingerprint
                )
                val fingerprintSchemas = mapOf(
                    "model" to "1.0",
                    "configuration" to "1.0",
                    "solver" to "1.0"
                )
                val artifact = SerializedSolution(
                    feasible = false,
                    optimal = false,
                    problemStatus = RemoteProblemStatus.UNKNOWN,
                    solutionPresence = RemoteSolutionPresence.NONE,
                    proofStatus = RemoteProofStatus.NONE,
                    terminationReason = RemoteTerminationReason.TIME_LIMIT,
                    schemaVersion = "2.0",
                    fingerprints = fingerprints,
                    fingerprintSchemas = fingerprintSchemas,
                    runId = "task",
                    attemptId = sliceId
                )
                val artifactDigest = digest(artifact)
                val resultRef = storage.put(
                    path = "tenant/result/task/$sliceId",
                    bytes = storage.json.encodeToString(
                        SerializedSolution.serializer(),
                        artifact.copy(artifactDigest = artifactDigest)
                    ).encodeToByteArray()
                )
                val checkpoint = OspfCpSnapshotExecutor(storage).exportCheckpoint(
                    payload = SolvePayload(
                        modelData = ModelData.raw(snapshot.encodeToByteArray(), "ospf-cp-snapshot-json")
                    ),
                    result = SolveResult(
                        feasible = false,
                        optimal = false,
                        objectiveValue = null,
                        gap = null,
                        elapsed = Duration.ZERO,
                        problemStatus = RemoteProblemStatus.UNKNOWN,
                        terminationReason = RemoteTerminationReason.TIME_LIMIT,
                        solutionPresence = RemoteSolutionPresence.NONE,
                        schemaVersion = "2.0",
                        fingerprints = fingerprints,
                        fingerprintSchemas = fingerprintSchemas,
                        runId = "task",
                        attemptId = sliceId,
                        resultRef = resultRef,
                        artifactDigest = artifactDigest
                    ),
                    tenantId = "tenant",
                    taskId = "task",
                    sliceId = sliceId
                )

                assertNotNull(checkpoint)
            }
        }
    }

    private fun historicalRuntimeFingerprints(): Set<String> {
        val pathNativeVersion = System.getProperty("ospf.scip.native.version")
            ?.takeUnless { it.isBlank() }
            ?: System.getenv("SCIP_VERSION")?.takeUnless { it.isBlank() }
            ?: "unknown"
        val pathPluginVersions = buildSet {
            ScipConstraintProgrammingSolver::class.java.`package`?.implementationVersion
                ?.takeUnless { it.isBlank() }
                ?.let(::add)
            ScipConstraintProgrammingSolver::class.java.protectionDomain?.codeSource?.location
                ?.toExternalForm()
                ?.takeUnless { it.isBlank() }
                ?.let(::add)
            if (isEmpty()) {
                add("unknown")
            }
        }
        val fingerprints = linkedSetOf<String>()
        pathPluginVersions.forEach { pluginVersion ->
            fingerprints += historicalRuntimeFingerprint(
                nativeVersion = pathNativeVersion,
                pluginVersion = pluginVersion,
                libraryIdentity = historicalLibraryIdentity()
            )
        }
        val contentNativeVersion = stableRuntimeVersionForTest(System.getProperty("ospf.scip.native.version"))
            ?: stableRuntimeVersionForTest(System.getProperty("scip.version"))
            ?: stableRuntimeVersionForTest(System.getenv("SCIP_VERSION"))
            ?: "unknown"
        val contentPluginVersion = stableRuntimeVersionForTest(System.getProperty("ospf.scip.plugin.version"))
            ?: stableRuntimeVersionForTest(System.getenv("OSPF_SCIP_PLUGIN_VERSION"))
            ?: stableRuntimeVersionForTest(ScipConstraintProgrammingSolver::class.java.`package`?.implementationVersion)
            ?: "1.1.0"
        historicalContentLibraryIdentities().forEach { libraryIdentity ->
            fingerprints += historicalRuntimeFingerprint(
                nativeVersion = contentNativeVersion,
                pluginVersion = contentPluginVersion,
                libraryIdentity = libraryIdentity
            )
        }
        return fingerprints
    }

    private fun historicalRuntimeFingerprint(
        nativeVersion: String,
        pluginVersion: String,
        libraryIdentity: String
    ): String {
        val canonical = listOf(
            "solverId=scip-cp",
            "backend=SCIP",
            "nativeVersion=$nativeVersion",
            "bindingVersion=jscip-1.0.0",
            "pluginVersion=$pluginVersion",
            "nativeLibraryIdentity=$libraryIdentity"
        ).joinToString("|")
        return "scip-runtime-1:${fingerprint(canonical)}"
    }

    private fun stableRuntimeVersionForTest(value: String?): String? {
        val normalized = value?.trim()?.takeUnless { it.isEmpty() } ?: return null
        return normalized.takeIf { candidate ->
            candidate.length <= 128 && candidate.all { character ->
                character.isLetterOrDigit() || character == '.' || character == '-' || character == '_' || character == '+'
            }
        }
    }

    private fun historicalContentLibraryIdentities(): Set<String> {
        val explicit = System.getProperty("ospf.scip.library")?.takeUnless { it.isBlank() }
        val library = if (explicit != null) {
            runCatching { Path.of(explicit).toAbsolutePath().normalize() }.getOrNull()
        } else {
            val fileName = System.mapLibraryName("jscip")
            System.getProperty("java.library.path")
                ?.split(java.io.File.pathSeparator)
                ?.asSequence()
                ?.mapNotNull { directory -> runCatching { Path.of(directory).resolve(fileName) }.getOrNull() }
                ?.firstOrNull { Files.isRegularFile(it) }
        }
        val digest = library?.let { historicalFileDigest(it) }
        val mode = if (explicit != null) "explicit" else "system"
        return if (digest == null) {
            setOf("$mode:unknown")
        } else {
            setOf("explicit:sha256:$digest", "system:sha256:$digest")
        }
    }

    private fun historicalFileDigest(path: Path): String? {
        return runCatching {
            val digest = MessageDigest.getInstance("SHA-256")
            Files.newInputStream(path).use { input ->
                val buffer = ByteArray(8192)
                while (true) {
                    val count = input.read(buffer)
                    if (count < 0) {
                        break
                    }
                    digest.update(buffer, 0, count)
                }
            }
            digest.digest().joinToString(separator = "") { byte -> "%02x".format(byte) }
        }.getOrNull()
    }

    private fun historicalLibraryIdentity(): String {
        val explicit = System.getProperty("ospf.scip.library")?.takeUnless { it.isBlank() }
        val library = if (explicit != null) {
            runCatching { Path.of(explicit).toAbsolutePath().normalize() }.getOrNull()
        } else {
            val fileName = System.mapLibraryName("jscip")
            System.getProperty("java.library.path")
                ?.split(java.io.File.pathSeparator)
                ?.asSequence()
                ?.mapNotNull { directory -> runCatching { Path.of(directory).resolve(fileName) }.getOrNull() }
                ?.firstOrNull { Files.isRegularFile(it) }
        }
        val prefix = if (explicit != null) "explicit" else "system"
        if (library == null) {
            return "$prefix:${System.getProperty("java.library.path") ?: "unknown"}"
        }
        return runCatching {
            val attributes = Files.readAttributes(
                library,
                java.nio.file.attribute.BasicFileAttributes::class.java
            )
            "$prefix:$library:size=${attributes.size()}:modified=${attributes.lastModifiedTime().toMillis()}"
        }.getOrDefault("$prefix:$library")
    }

    private fun digest(solution: SerializedSolution): String {
        val json = Json { encodeDefaults = true }
        val serialized = json.encodeToString(
            SerializedSolution.serializer(),
            solution.copy(artifactDigest = null)
        )
        val canonical = json.encodeToString(
            SerializedSolution.serializer(),
            json.decodeFromString(SerializedSolution.serializer(), serialized)
        )
        return MessageDigest.getInstance("SHA-256")
            .digest(canonical.toByteArray(Charsets.UTF_8))
            .joinToString(separator = "") { byte -> "%02x".format(byte) }
    }

    private fun snapshotJson(): String {
        val model = ConstraintProgrammingModel("checkpoint-model")
        return try {
            ConstraintProgrammingSnapshotCodec.encode(model.snapshot().value!!).value!!
        } finally {
            model.close()
        }
    }

    private fun objectiveSnapshotJson(): Pair<String, String> {
        val model = ConstraintProgrammingModel("objective-model")
        return try {
            val variable = fuookami.ospf.kotlin.core.variable.IntVar("objective-x")
            assertTrue(model.registerVariable(variable, IntegerDomain.interval(0, 1).value!!).ok)
            assertTrue(model.minimize(ConstraintProgrammingExpression.Variable(variable)).ok)
            val snapshot = model.snapshot().value!!
            ConstraintProgrammingSnapshotCodec.encode(snapshot).value!! to snapshot.variables.single().id.value
        } finally {
            model.close()
        }
    }

    private fun configurationCanonical(timeLimitMs: String? = null): String {
        return buildString {
            appendCanonical("timeLimitMs", timeLimitMs)
            appendCanonical("solutionLimit", null)
            appendCanonical("mipGapTolerance", null)
            appendCanonical("threads", "8")
        }
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

    private fun legacyDelimiterCanonical(config: SolverConfig): String {
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

    private fun StringBuilder.appendCanonical(name: String, value: String?) {
        append(name.length)
            .append(':')
            .append(name)
            .append(value?.length ?: -1)
            .append(':')
            .append(value ?: "")
    }

    private fun fingerprint(value: String): String {
        return MessageDigest.getInstance("SHA-256")
            .digest(value.toByteArray(Charsets.UTF_8))
            .joinToString(separator = "") { byte -> "%02x".format(byte) }
    }

    private class RecordingStorage : ObjectStoragePort {
        val json = Json { encodeDefaults = true }
        private val objects = mutableMapOf<ObjectPath, ByteArray>()

        override suspend fun put(
            path: ObjectPath,
            bytes: ByteArray,
            metadata: Map<String, String>
        ): ObjectRef {
            objects[path] = bytes
            return ObjectRef(path = path)
        }

        override suspend fun get(ref: ObjectRef): ByteArray? = objects[ref.path]

        override suspend fun delete(ref: ObjectRef): Boolean = objects.remove(ref.path) != null

        override suspend fun exists(ref: ObjectRef): Boolean = objects.containsKey(ref.path)
    }
}
