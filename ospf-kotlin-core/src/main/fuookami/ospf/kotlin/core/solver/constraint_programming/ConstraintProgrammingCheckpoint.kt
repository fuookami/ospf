/** CP checkpoint capability assessment. / CP checkpoint 能力评估。 */
package fuookami.ospf.kotlin.core.solver.constraint_programming

import fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingModelSnapshot
import fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingSnapshotCodec
import fuookami.ospf.kotlin.core.solver.output.ConstraintProgrammingSolution
import fuookami.ospf.kotlin.core.solver.report.SolverDescriptor
import fuookami.ospf.kotlin.core.solver.report.SolverModelType
import fuookami.ospf.kotlin.utils.error.ErrorCode
import fuookami.ospf.kotlin.utils.functional.Failed
import fuookami.ospf.kotlin.utils.functional.Ret
import fuookami.ospf.kotlin.utils.functional.ok

/** Checkpoint support level. / checkpoint 支持级别。 */
enum class ConstraintProgrammingCheckpointSupport {
    Native,
    RebuildFromSnapshot,
    Unsupported
}

/** Structured capability assessment. / 结构化能力评估。 */
data class ConstraintProgrammingCapabilityAssessment(
    val solverId: String,
    val support: ConstraintProgrammingCheckpointSupport,
    val canCheckpoint: Boolean,
    val canResume: Boolean,
    val reasons: List<String>
)

/** Snapshot plus optional incumbent used for portable checkpoint evaluation. */
data class ConstraintProgrammingCheckpoint(
    val schema: Int,
    val modelName: String,
    val snapshotJson: String,
    val solverId: String,
    val incumbent: ConstraintProgrammingSolution? = null
)

/**
 * / 评估原生 checkpoint 能力并提供可移植 snapshot checkpoint。 / Assess native checkpointing and provide a portable snapshot checkpoint.
 */
object ConstraintProgrammingCheckpointSupportEvaluator {
    /** Assess one solver descriptor. / 评估一个求解器描述符。 */
    fun assess(descriptor: SolverDescriptor): ConstraintProgrammingCapabilityAssessment {
        val capabilities = descriptor.capabilities
        val cpSupported = SolverModelType.CP in capabilities.modelTypes
        val native = cpSupported && capabilities.checkpoint && capabilities.resume
        val reasons = ArrayList<String>()
        if (!cpSupported) {
            reasons += "求解器未声明 CP 模型 / Solver does not declare CP models"
        }
        if (!capabilities.checkpoint) {
            reasons += "后端未声明 checkpoint / Backend does not declare checkpoint support"
        }
        if (!capabilities.resume) {
            reasons += "后端未声明 resume / Backend does not declare resume support"
        }
        return ConstraintProgrammingCapabilityAssessment(
            solverId = descriptor.solverId,
            support = when {
                native -> ConstraintProgrammingCheckpointSupport.Native
                cpSupported -> ConstraintProgrammingCheckpointSupport.RebuildFromSnapshot
                else -> ConstraintProgrammingCheckpointSupport.Unsupported
            },
            canCheckpoint = cpSupported,
            canResume = native,
            reasons = reasons
        )
    }

    /** Capture a portable checkpoint; native state is intentionally excluded. */
    fun capture(
        snapshot: ConstraintProgrammingModelSnapshot,
        descriptor: SolverDescriptor,
        incumbent: ConstraintProgrammingSolution? = null
    ): Ret<ConstraintProgrammingCheckpoint> {
        val assessment = assess(descriptor)
        if (assessment.support == ConstraintProgrammingCheckpointSupport.Unsupported) {
            return Failed(
                ErrorCode.Other,
                "求解器不支持 CP checkpoint：${descriptor.solverId} / Solver does not support CP checkpoint: ${descriptor.solverId}"
            )
        }
        val encoded = ConstraintProgrammingSnapshotCodec.encode(snapshot)
        if (encoded.failed) {
            return when (encoded) {
                is Failed -> Failed(encoded.error)
                is fuookami.ospf.kotlin.utils.functional.Fatal -> fuookami.ospf.kotlin.utils.functional.Fatal(encoded.errors)
                else -> Failed(ErrorCode.ApplicationError, "CP checkpoint 编码状态无效 / Invalid CP checkpoint encoding state")
            }
        }
        return ok(
            ConstraintProgrammingCheckpoint(
                schema = 1,
                modelName = snapshot.name,
                snapshotJson = encoded.value!!,
                solverId = descriptor.solverId,
                incumbent = incumbent
            )
        )
    }
}
