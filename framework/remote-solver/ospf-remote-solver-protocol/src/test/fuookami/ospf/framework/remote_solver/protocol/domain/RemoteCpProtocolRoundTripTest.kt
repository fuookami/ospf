package fuookami.ospf.framework.remote_solver.protocol.domain

import java.security.MessageDigest
import kotlin.time.Duration
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.put

/**
 * CP 远程协议 round-trip 回归测试。
 * CP remote protocol round-trip regression tests.
 */
class RemoteCpProtocolRoundTripTest {
    private val json = Json {
        encodeDefaults = true
        ignoreUnknownKeys = false
    }

    /**
     * 验证 CP 结果的稳定 ID 和 Int64 不经过浮点转换。
     * Verifies stable CP result IDs and exact Int64 transport without floating-point conversion.
     */
    @Test
    fun serializedCpSolutionPreservesInt64AndIntervals() {
        listOf(9_007_199_254_740_993L, Long.MAX_VALUE, Long.MIN_VALUE).forEach { objective ->
            val solution = SerializedSolution(
                feasible = true,
                optimal = false,
                objectiveValue = null,
                objectiveValueInt64 = objective,
                variableValuesById = mapOf("x" to Long.MAX_VALUE, "y" to Long.MIN_VALUE),
                intervalValues = mapOf(
                    "job" to SerializedIntervalValue(start = 1L, size = 2L, end = 3L, present = true)
                ),
                problemStatus = RemoteProblemStatus.FEASIBLE,
                solutionPresence = RemoteSolutionPresence.INCUMBENT
            )

            val decoded = json.decodeFromString(
                SerializedSolution.serializer(),
                json.encodeToString(SerializedSolution.serializer(), solution)
            )

            assertEquals(null, decoded.objectiveValue)
            assertEquals(objective, decoded.objectiveValueInt64)
            assertEquals(Long.MAX_VALUE, decoded.variableValuesById["x"])
            assertEquals(Long.MIN_VALUE, decoded.variableValuesById["y"])
            assertEquals(SerializedIntervalValue(1L, 2L, 3L, true), decoded.intervalValues["job"])
            assertEquals(RemoteProblemStatus.FEASIBLE, decoded.problemStatus)
            assertEquals(RemoteSolutionPresence.INCUMBENT, decoded.solutionPresence)

            val result = SolveResult(
                feasible = true,
                optimal = false,
                objectiveValue = null,
                gap = null,
                elapsed = Duration.ZERO,
                objectiveValueInt64 = objective
            )
            val decodedResult = json.decodeFromString(
                SolveResult.serializer(),
                json.encodeToString(SolveResult.serializer(), result)
            )

            assertEquals(null, decodedResult.objectiveValue)
            assertEquals(objective, decodedResult.objectiveValueInt64)
        }
    }

    /**
     * 验证 CP format 被识别为 CP，而不是 UNKNOWN。
     * Verifies that the CP format is classified as CP rather than UNKNOWN.
     */
    @Test
    fun cpFormatIsClassifiedExplicitly() {
        val data = ModelData.raw("{}".encodeToByteArray(), "ospf-cp-snapshot-json")

        assertEquals(NormalizedModelType.CP, data.modelType)
    }

    /**
     * 验证对象引用的 path、version 和 etag 均能往返。
     * Verifies that ObjectRef path, version, and etag all survive a JSON round-trip.
     */
    @Test
    fun objectRefPreservesVersionAndEtag() {
        val reference = ObjectRef.of(
            path = "tenant-a/result/task-1/slice-1",
            version = "v2",
            etag = "sha256:abc"
        )
        val decoded = json.decodeFromString(
            ObjectRef.serializer(),
            json.encodeToString(ObjectRef.serializer(), reference)
        )

        assertEquals(reference, decoded)
    }

    /**
     * 验证跨仓库 canonical CP v2 fixture 的字段和精确数值。
     * Verifies fields and exact values of the cross-repository canonical CP v2 fixture.
     */
    @Test
    fun canonicalCpV2FixtureDecodesWithoutFloatingPointConversion() {
        val fixture = checkNotNull(javaClass.getResource("/fixtures/remote-cp-result-v2.json"))
            .readText()
        val solution = json.decodeFromString(SerializedSolution.serializer(), fixture)

        assertEquals("2.0", solution.schemaVersion)
        assertEquals(9_007_199_254_740_993L, solution.objectiveValueInt64)
        assertEquals(9_007_199_254_740_993L, solution.variableValuesById["x"])
        assertEquals(SerializedIntervalValue(1L, 2L, 3L, true), solution.intervalValues["job"])
        assertEquals(RemoteTerminationReason.TIME_LIMIT, solution.terminationReason)
        assertEquals("1.0", solution.fingerprintSchemas["model"])
        assertEquals("0.5", solution.statistics["bestBound"])
    }

    /**
     * 验证线性/二次共用的 v2 报告 fixture 在服务端 protocol 模块可读取。
     * Verifies that the shared v2 linear/quadratic report fixture is readable by the server protocol module.
     */
    @Test
    fun canonicalLinearV2FixtureDecodesSharedReportFields() {
        val fixture = checkNotNull(javaClass.getResource("/fixtures/remote-linear-result-v2.json"))
            .readText()
        val solution = json.decodeFromString(SerializedSolution.serializer(), fixture)

        assertEquals("2.0", solution.schemaVersion)
        assertEquals(12.5, solution.objectiveValue?.toDouble())
        assertEquals(0.2, solution.gap?.toDouble())
        assertEquals(listOf(2.0, 3.0), solution.variableValues.map { it.toDouble() })
        assertEquals(RemoteSolutionPresence.INCUMBENT, solution.solutionPresence)
        assertEquals(RemoteProofStatus.CLAIMED, solution.proofStatus)
        assertEquals(RemoteTerminationReason.TIME_LIMIT, solution.terminationReason)
        assertEquals("2.0", solution.fingerprintSchemas["solver"])
        assertEquals("10.5", solution.statistics["bestBound"])
        assertEquals("run-linear-1", solution.runId)
        assertEquals("attempt-linear-1", solution.attemptId)
    }

    /**
     * 验证跨仓库线性模型 fixture 保留稳定身份元数据。
     * Verifies that the cross-repository linear model fixture preserves stable identity metadata.
     */
    @Test
    fun canonicalLinearModelFixturePreservesIdentityMetadata() {
        val fixture = checkNotNull(javaClass.getResource("/fixtures/remote-linear-model-v2.json"))
            .readText()
        val model = json.decodeFromString(SerializedLinearModel.serializer(), fixture)

        assertEquals("fixture-model", model.identityNamespace)
        assertEquals("1.0", model.identitySchemaVersion)
        assertEquals("variable:x", model.variables.single().identityId)
        assertEquals("STABLE", model.variables.single().identityScope)
        assertEquals("demand", model.variables.single().identityOriginKind)
        assertEquals(
            listOf(
                SerializedModelElementOrigin("demand", "x"),
                SerializedModelElementOrigin("source", "x")
            ),
            model.variables.single().identityProvenance
        )
        assertEquals("constraint:capacity", model.constraints.single().identityId)
        assertEquals("capacity", model.constraints.single().identityOriginKind)
        assertEquals(
            listOf(
                SerializedModelElementOrigin("capacity", "machine-1"),
                SerializedModelElementOrigin("source", "machine-1")
            ),
            model.constraints.single().identityProvenance
        )
        assertEquals("objective:cost", model.objective.identityId)
        assertEquals("total", model.objective.identityOriginKey)
        assertEquals(
            listOf(
                SerializedModelElementOrigin("cost", "total"),
                SerializedModelElementOrigin("source", "total")
            ),
            model.objective.identityProvenance
        )

        val roundTripped = json.decodeFromString(
            SerializedLinearModel.serializer(),
            json.encodeToString(SerializedLinearModel.serializer(), model)
        )
        assertEquals(model, roundTripped)
    }

    /**
     * 验证旧 CP result artifact 可读取但不会凭旧布尔字段升级证明。
     * Verifies that the legacy CP result artifact remains readable without upgrading proof claims from legacy booleans.
     */
    @Test
    fun legacyCpResultFixtureRemainsReadableWithoutModernProofFields() {
        val fixture = checkNotNull(javaClass.getResource("/fixtures/remote-cp-result-v1.json"))
            .readText()
        val solution = json.decodeFromString(SerializedSolution.serializer(), fixture)

        assertEquals("1.0", solution.schemaVersion)
        assertEquals(true, solution.feasible)
        assertEquals(true, solution.optimal)
        assertEquals(12.5, solution.objectiveValue?.toDouble())
        assertEquals(null, solution.problemStatus)
        assertEquals(null, solution.terminationReason)
        assertEquals(null, solution.proofStatus)
        assertEquals(emptyMap(), solution.variableValuesById)
    }

    /**
     * Verifies portable checkpoint digest and legacy v1 compatibility.
     * 验证 portable checkpoint 摘要校验与 legacy v1 兼容读取。
     */
    @Test
    fun portableCheckpointSupportsLegacyReadAndRejectsCorruption() {
        val envelope = PortableCheckpointEnvelope(
            checkpointId = "cp-1",
            modelFingerprint = "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a",
            createdAtEpochMs = 1L,
            snapshotJson = "{}"
        )
        val encoded = PortableCheckpointCodec.encode(envelope)
        assertEquals("cp-1", PortableCheckpointCodec.decodeOrNull(encoded)!!.checkpointId)
        assertEquals(null, PortableCheckpointCodec.decodeOrNull(encoded.replace("cp-1", "cp-2")))

        val legacy = buildJsonObject {
            put("schema", 1)
            put("modelName", "legacy")
            put("solverId", "solver")
            put(
                "snapshotJson",
                buildJsonObject {
                    put("name", "legacy-model")
                    put("identitySchemaVersion", "1.0")
                    put("identityNamespace", "legacy-space")
                }.toString()
            )
        }.toString()
        val decoded = PortableCheckpointCodec.decodeCompatibleOrNull(legacy)!!
        assertEquals("legacy-v1", decoded.checkpointId)
        assertEquals("legacy", decoded.modelName)
        assertEquals("legacy-space", decoded.identityNamespace)
        assertEquals(null, decoded.solverFingerprint)
    }

    /**
     * 验证 v2 envelope 不能伪装成 legacy-v1 或携带不匹配的内嵌 snapshot。
     * Verifies that a v2 envelope cannot masquerade as legacy-v1 or carry a mismatched embedded snapshot.
     */
    @Test
    fun v2CheckpointRejectsLegacySourceAndEmbeddedModelMismatch() {
        val base = PortableCheckpointEnvelope(
            checkpointId = "cp-spoof",
            modelFingerprint = sha256("{}"),
            createdAtEpochMs = 1L,
            snapshotJson = "{}"
        )
        val spoof = encodedWithDigest(base.copy(sourceFormat = "legacy-v1"))
        assertEquals(null, PortableCheckpointCodec.decodeOrNull(spoof))

        val mismatch = encodedWithDigest(
            base.copy(
                checkpointId = "cp-mismatch",
                snapshotJson = "{\"name\":\"other\"}"
            )
        )
        assertEquals(null, PortableCheckpointCodec.decodeOrNull(mismatch))
    }

    /**
     * 验证未来 checkpoint 主版本不会降级为 legacy v1。
     * Verifies that a future checkpoint major version never downgrades to legacy v1.
     */
    @Test
    fun futureCheckpointMajorVersionIsRejected() {
        val future = encodedWithDigest(
            PortableCheckpointEnvelope(
                schemaVersion = "3.0",
                checkpointId = "future-v3",
                modelFingerprint = sha256("{}"),
                createdAtEpochMs = 1L,
                snapshotJson = "{}"
            )
        )

        assertEquals(null, PortableCheckpointCodec.decodeCompatibleOrNull(future))
    }

    /**
     * 验证 masterFingerprint 加入前发布的 v2 Benders 摘要仍可读取。
     * Verifies that published V2 Benders digests from before masterFingerprint remain readable.
     */
    @Test
    fun publishedV2BendersDigestWithoutMasterFingerprintRemainsReadable() {
        val base = PortableCheckpointEnvelope(
            checkpointId = "cp-benders-legacy",
            modelFingerprint = sha256("{}"),
            createdAtEpochMs = 1L,
            snapshotJson = "{}",
            benders = PortableConstraintProgrammingBendersState(
                iteration = 2L,
                masterIncumbent = "3.0"
            )
        )
        val encoded = PortableCheckpointCodec.encode(base)
        val root = json.parseToJsonElement(encoded).jsonObject
        val legacyBenders = JsonObject(root.getValue("benders").jsonObject.toMutableMap().apply {
            remove("masterFingerprint")
        })
        val unsigned = JsonObject(root.toMutableMap().apply {
            put("benders", legacyBenders)
            put("integritySha256", JsonPrimitive(""))
        })
        val digest = sha256(unsigned.toString())
        val legacy = JsonObject(unsigned.toMutableMap().apply {
            put("integritySha256", JsonPrimitive(digest))
        }).toString()

        val decoded = PortableCheckpointCodec.decodeOrNull(legacy)
        assertEquals("cp-benders-legacy", decoded?.checkpointId)
        assertEquals(null, decoded?.benders?.masterFingerprint)
        assertEquals("3.0", decoded?.benders?.masterIncumbent)
    }

    private fun encodedWithDigest(envelope: PortableCheckpointEnvelope): String {
        val unsigned = envelope.copy(integritySha256 = "")
        val digest = sha256(json.encodeToString(PortableCheckpointEnvelope.serializer(), unsigned))
        return json.encodeToString(
            PortableCheckpointEnvelope.serializer(),
            unsigned.copy(integritySha256 = digest)
        )
    }

    private fun sha256(value: String): String {
        return MessageDigest.getInstance("SHA-256")
            .digest(value.toByteArray(Charsets.UTF_8))
            .joinToString(separator = "") { byte -> "%02x".format(byte) }
    }
}
