/** Portable checkpoint JSON codec. / 可移植 checkpoint JSON 编解码器。 */
package fuookami.ospf.framework.remote_solver.protocol.domain

import java.security.MessageDigest
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.intOrNull
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive

/**
 * Encodes and verifies portable checkpoint envelopes.
 * 编码并验证可移植 checkpoint envelope。
 */
object PortableCheckpointCodec {
    private const val CURRENT_SCHEMA = "2.0"
    private val V2_MARKER_FIELDS = setOf(
        "schemaVersion",
        "sourceFormat",
        "migratedFromLegacy",
        "checkpointId",
        "identitySchemaVersion",
        "identityNamespace",
        "modelFingerprint",
        "configurationFingerprint",
        "solverFingerprint",
        "runId",
        "attemptId",
        "parentCheckpointId",
        "createdAtEpochMs",
        "incumbent",
        "bestBound",
        "gap",
        "assumptions",
        "conflicts",
        "benders",
        "integritySha256"
    )
    private val LEGACY_V1_FIELDS = setOf(
        "schema",
        "modelName",
        "solverId",
        "snapshotJson"
    )
    private val json = Json {
        encodeDefaults = true
        ignoreUnknownKeys = false
    }

    /** Benders state shape used by published V2 documents before masterFingerprint was added.
     * masterFingerprint 加入前已发布的 V2 Benders 状态形状。
     */
    @kotlinx.serialization.Serializable
    private data class LegacyBendersState(
        val iteration: Long,
        val masterIncumbent: String? = null,
        val masterBestBound: String? = null,
        val cuts: List<PortableConstraintProgrammingCut> = emptyList(),
        val trace: List<String> = emptyList(),
        val assumptions: List<String> = emptyList(),
        val fixedBindings: Map<String, Long> = emptyMap(),
        val conflicts: List<PortableConstraintProgrammingConflict> = emptyList(),
        val convergenceVerified: Boolean = false,
        val subproblemModelFingerprint: String? = null
    )

    /** V2 envelope serializer retaining the published pre-masterFingerprint shape.
     * 保留 masterFingerprint 之前已发布形状的 V2 envelope serializer。
     */
    @kotlinx.serialization.Serializable
    private data class LegacyEnvelope(
        val schemaVersion: String = CURRENT_SCHEMA,
        val sourceFormat: String = "v2",
        val migratedFromLegacy: Boolean = false,
        val checkpointId: String,
        val identitySchemaVersion: String = "1.0",
        val identityNamespace: String = "model-local",
        val modelName: String = "remote-cp",
        val modelFingerprint: String,
        val configurationFingerprint: String? = null,
        val solverFingerprint: String? = null,
        val runId: String? = null,
        val attemptId: String? = null,
        val parentCheckpointId: String? = null,
        val createdAtEpochMs: Long,
        val snapshotJson: String,
        val incumbent: PortableConstraintProgrammingIncumbent? = null,
        val bestBound: String? = null,
        val gap: String? = null,
        val assumptions: List<String> = emptyList(),
        val conflicts: List<PortableConstraintProgrammingConflict> = emptyList(),
        val benders: LegacyBendersState? = null,
        val integritySha256: String = ""
    )

    /**
     * Encode an envelope with a refreshed digest.
     * 编码 envelope 并刷新完整性摘要。
     *
     * @param envelope checkpoint envelope / checkpoint envelope
     * @return JSON 文本 / JSON text
     */
    fun encode(envelope: PortableCheckpointEnvelope): String {
        val migratedFromLegacy = envelope.sourceFormat == "legacy-v1" || envelope.migratedFromLegacy
        val normalized = envelope.copy(
            schemaVersion = CURRENT_SCHEMA,
            sourceFormat = "v2",
            migratedFromLegacy = migratedFromLegacy,
            checkpointId = if (migratedFromLegacy) "legacy-v1-migrated" else envelope.checkpointId,
            integritySha256 = digest(
                envelope.copy(
                    schemaVersion = CURRENT_SCHEMA,
                    sourceFormat = "v2",
                    migratedFromLegacy = migratedFromLegacy,
                    checkpointId = if (migratedFromLegacy) "legacy-v1-migrated" else envelope.checkpointId,
                    integritySha256 = ""
                )
            )
        )
        return json.encodeToString(PortableCheckpointEnvelope.serializer(), normalized)
    }

    /**
     * Decode and verify an envelope.
     * 解码并验证 envelope。
     *
     * @param encoded JSON 文本 / JSON text
     * @return verified envelope, or null when invalid / 已验证 envelope；无效时返回 null
     */
    fun decodeOrNull(encoded: String): PortableCheckpointEnvelope? {
        return runCatching {
            val envelope = json.decodeFromString(PortableCheckpointEnvelope.serializer(), encoded)
            if (envelope.schemaVersion != CURRENT_SCHEMA) {
                return@runCatching null
            }
            if (envelope.sourceFormat != "v2") {
                return@runCatching null
            }
            if (!envelope.migratedFromLegacy && envelope.checkpointId == "legacy-v1") {
                return@runCatching null
            }
            if (envelope.migratedFromLegacy && envelope.checkpointId != "legacy-v1-migrated") {
                return@runCatching null
            }
            if (envelope.integritySha256 != digest(envelope.copy(integritySha256 = ""))) {
                return@runCatching decodeLegacyV2OrNull(encoded)
            }
            if (sha256(envelope.snapshotJson) != envelope.modelFingerprint) {
                return@runCatching null
            }
            envelope
        }.getOrNull()
    }

    /**
     * Decodes v2 or the legacy snapshot-only v1 envelope.
     * 解码 v2 或 legacy 仅 snapshot 的 v1 envelope。
     *
     * @param encoded checkpoint JSON / Checkpoint JSON
     * @return verified envelope, or null when invalid / 已验证 envelope；无效时返回 null
     */
    fun decodeCompatibleOrNull(encoded: String): PortableCheckpointEnvelope? {
        decodeOrNull(encoded)?.let { return it }
        return runCatching {
            val root = json.parseToJsonElement(encoded).jsonObject
            if (root.keys.any { it in V2_MARKER_FIELDS }) {
                return@runCatching null
            }
            if (root.keys.any { it !in LEGACY_V1_FIELDS }) {
                return@runCatching null
            }
            if (root["schema"]?.jsonPrimitive?.intOrNull != 1) {
                return@runCatching null
            }
            val snapshotJson = root["snapshotJson"]?.jsonPrimitive?.content
                ?: return@runCatching null
            val snapshotRoot = json.parseToJsonElement(snapshotJson).jsonObject
            val envelope = PortableCheckpointEnvelope(
                checkpointId = "legacy-v1",
                sourceFormat = "legacy-v1",
                migratedFromLegacy = false,
                identitySchemaVersion = snapshotRoot["identitySchemaVersion"]?.jsonPrimitive?.content ?: "1.0",
                identityNamespace = snapshotRoot["identityNamespace"]?.jsonPrimitive?.content ?: "model-local",
                modelName = root["modelName"]?.jsonPrimitive?.content
                    ?: snapshotRoot["name"]?.jsonPrimitive?.content
                    ?: "legacy",
                modelFingerprint = sha256(snapshotJson),
                // Legacy v1 did not define the canonical solver fingerprint. / Legacy v1 没有定义规范化求解器指纹。
                solverFingerprint = null,
                createdAtEpochMs = 0L,
                snapshotJson = snapshotJson
            )
            envelope
        }.getOrNull()
    }

    private fun digest(envelope: PortableCheckpointEnvelope): String {
        val bytes = json.encodeToString(PortableCheckpointEnvelope.serializer(), envelope)
            .toByteArray(Charsets.UTF_8)
        return MessageDigest.getInstance("SHA-256")
            .digest(bytes)
            .joinToString(separator = "") { byte -> "%02x".format(byte) }
    }

    private fun digest(envelope: LegacyEnvelope): String {
        val bytes = json.encodeToString(LegacyEnvelope.serializer(), envelope)
            .toByteArray(Charsets.UTF_8)
        return MessageDigest.getInstance("SHA-256")
            .digest(bytes)
            .joinToString(separator = "") { byte -> "%02x".format(byte) }
    }

    private fun decodeLegacyV2OrNull(encoded: String): PortableCheckpointEnvelope? {
        return runCatching {
            val legacy = json.decodeFromString(LegacyEnvelope.serializer(), encoded)
            if (legacy.schemaVersion != CURRENT_SCHEMA || legacy.sourceFormat != "v2" ||
                (!legacy.migratedFromLegacy && legacy.checkpointId == "legacy-v1") ||
                (legacy.migratedFromLegacy && legacy.checkpointId != "legacy-v1-migrated") ||
                sha256(legacy.snapshotJson) != legacy.modelFingerprint ||
                legacy.integritySha256.isBlank() ||
                    legacy.integritySha256 != digest(legacy.copy(integritySha256 = ""))
            ) {
                return@runCatching null
            }
            PortableCheckpointEnvelope(
                schemaVersion = legacy.schemaVersion,
                sourceFormat = legacy.sourceFormat,
                migratedFromLegacy = legacy.migratedFromLegacy,
                checkpointId = legacy.checkpointId,
                identitySchemaVersion = legacy.identitySchemaVersion,
                identityNamespace = legacy.identityNamespace,
                modelName = legacy.modelName,
                modelFingerprint = legacy.modelFingerprint,
                configurationFingerprint = legacy.configurationFingerprint,
                solverFingerprint = legacy.solverFingerprint,
                runId = legacy.runId,
                attemptId = legacy.attemptId,
                parentCheckpointId = legacy.parentCheckpointId,
                createdAtEpochMs = legacy.createdAtEpochMs,
                snapshotJson = legacy.snapshotJson,
                incumbent = legacy.incumbent,
                bestBound = legacy.bestBound,
                gap = legacy.gap,
                assumptions = legacy.assumptions,
                conflicts = legacy.conflicts,
                benders = legacy.benders?.let { state ->
                    PortableConstraintProgrammingBendersState(
                        iteration = state.iteration,
                        masterIncumbent = state.masterIncumbent,
                        masterBestBound = state.masterBestBound,
                        cuts = state.cuts,
                        trace = state.trace,
                        assumptions = state.assumptions,
                        fixedBindings = state.fixedBindings,
                        conflicts = state.conflicts,
                        convergenceVerified = state.convergenceVerified,
                        masterFingerprint = null,
                        subproblemModelFingerprint = state.subproblemModelFingerprint
                    )
                },
                integritySha256 = legacy.integritySha256
            )
        }.getOrNull()
    }

    private fun sha256(value: String): String {
        return MessageDigest.getInstance("SHA-256")
            .digest(value.toByteArray(Charsets.UTF_8))
            .joinToString(separator = "") { byte -> "%02x".format(byte) }
    }
}
