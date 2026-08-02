/** Portable remote checkpoint v2 models. / 可移植远程 checkpoint v2 模型。 */
package fuookami.ospf.framework.remote_solver.protocol.domain

import kotlinx.serialization.Serializable

/**
 * Versioned checkpoint envelope that excludes native solver handles. / 排除 native 求解器句柄的版本化 checkpoint envelope。
 *
 * @property schemaVersion envelope schema 版本 / Envelope schema version
 * @property sourceFormat checkpoint 来源格式 / Source format of the checkpoint
 * @property migratedFromLegacy 是否由 legacy v1 迁移 / Whether this envelope was migrated from legacy v1
 * @property checkpointId checkpoint 标识 / Checkpoint identifier
 * @property identitySchemaVersion 身份 schema 版本 / Identity schema version
 * @property identityNamespace 身份命名空间 / Identity namespace
 * @property modelName 模型名称 / Model name
 * @property modelFingerprint 模型指纹 / Model fingerprint
 * @property configurationFingerprint 配置指纹 / Configuration fingerprint
 * @property solverFingerprint 求解器指纹 / Solver fingerprint
 * @property runId 运行标识 / Run identifier
 * @property attemptId 尝试标识 / Attempt identifier
 * @property parentCheckpointId 父 checkpoint 标识 / Parent checkpoint identifier
 * @property createdAtEpochMs 创建时间 / Creation time
 * @property snapshotJson snapshot JSON / Snapshot JSON
 * @property incumbent 已验证 incumbent / Validated incumbent
 * @property bestBound 历史 bound / Historical bound
 * @property gap 历史 gap / Historical gap
 * @property assumptions assumption 标识 / Assumption identifiers
 * @property conflicts 冲突证据 / Conflict evidence
 * @property benders Benders 状态 / Benders state
 * @property integritySha256 完整性摘要 / Integrity digest
 */
@Serializable
data class PortableCheckpointEnvelope(
    val schemaVersion: String = "2.0",
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
    val benders: PortableConstraintProgrammingBendersState? = null,
    val integritySha256: String = ""
)

/**
 * Portable incumbent representation. / 可移植 incumbent 表示。
 *
 * @property valuesById 稳定变量值 / Stable variable values
 * @property intervalsById 稳定 interval 值 / Stable interval values
 * @property objective 目标值 / Objective value
 */
@Serializable
data class PortableConstraintProgrammingIncumbent(
    val valuesById: Map<String, Long> = emptyMap(),
    val intervalsById: Map<String, PortableConstraintProgrammingIntervalValue> = emptyMap(),
    val objective: String? = null
)

/** Portable interval value. / 可移植 interval 值。
 *
 * @property start 开始值 / Start value
 * @property size 长度值 / Size value
 * @property end 结束值 / End value
 * @property present 是否存在 / Whether the interval is present
 */
@Serializable
data class PortableConstraintProgrammingIntervalValue(
    val start: Long,
    val size: Long,
    val end: Long,
    val present: Boolean = true
)

/** Portable conflict evidence. / 可移植冲突证据。
 *
 * @property validity 证据有效性 / Evidence validity
 * @property minimality 证据最小性 / Evidence minimality
 * @property memberIds 冲突成员标识 / Conflict member identifiers
 * @property assumptionIds 参与冲突的 assumption 标识 / Assumption identifiers participating in the conflict
 * @property provenance 来源摘要 / Provenance summary
 */
@Serializable
data class PortableConstraintProgrammingConflict(
    val validity: String,
    val minimality: String,
    val memberIds: List<String> = emptyList(),
    val assumptionIds: List<String> = emptyList(),
    val provenance: Map<String, String> = emptyMap()
)

/** Portable Benders state. / 可移植 Benders 状态。
 *
 * @property iteration 当前迭代 / Current iteration
 * @property masterIncumbent 主问题 incumbent / Master incumbent
 * @property masterBestBound 主问题最佳界 / Master best bound
 * @property cuts 已接受的 cuts / Accepted cuts
 * @property trace 迭代轨迹 / Iteration trace
 * @property assumptions 已复验的 assumption 标识 / Revalidated assumption identifiers
 * @property fixedBindings 已复验的固定绑定 / Revalidated fixed bindings
 * @property conflicts 已复验的冲突证据 / Revalidated conflict evidence
 * @property convergenceVerified 历史收敛标记 / Historical convergence marker
 * @property masterFingerprint 主问题模型指纹 / Master model fingerprint
 * @property subproblemModelFingerprint 子问题模型指纹 / Subproblem model fingerprint
 */
@Serializable
data class PortableConstraintProgrammingBendersState(
    val iteration: Long,
    val masterIncumbent: String? = null,
    val masterBestBound: String? = null,
    val cuts: List<PortableConstraintProgrammingCut> = emptyList(),
    val trace: List<String> = emptyList(),
    val assumptions: List<String> = emptyList(),
    val fixedBindings: Map<String, Long> = emptyMap(),
    val conflicts: List<PortableConstraintProgrammingConflict> = emptyList(),
    val convergenceVerified: Boolean = false,
    val masterFingerprint: String? = null,
    val subproblemModelFingerprint: String? = null
)

/** Versioned primitive cut payload. / 版本化基础类型 cut payload。
 *
 * @property id 稳定 cut 标识 / Stable cut identifier
 * @property schemaVersion cut schema / Cut schema
 * @property validity 有效性 / Validity
 * @property provenance 来源摘要 / Provenance summary
 * @property payload 基础类型 payload / Primitive payload
 */
@Serializable
data class PortableConstraintProgrammingCut(
    val id: String,
    val schemaVersion: String,
    val validity: String,
    val provenance: Map<String, String> = emptyMap(),
    val payload: Map<String, String> = emptyMap()
)
