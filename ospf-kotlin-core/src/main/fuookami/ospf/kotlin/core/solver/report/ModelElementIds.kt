/** Diagnostic model-element identity helpers. / 诊断模型元素身份辅助函数。 */
package fuookami.ospf.kotlin.core.solver.report

import fuookami.ospf.kotlin.core.model.basic.Variable
import fuookami.ospf.kotlin.core.model.intermediate.LinearTriadModelView
import fuookami.ospf.kotlin.core.model.intermediate.QuadraticTetradModelView

/**
 * Resolve the variable identity available to a diagnostic provider. /
 * 解析诊断提供者当前可用的变量身份。
 *
 * Origin-backed IDs are reusable inside the existing model pipeline. The fallback is explicitly
 * model-local until OSPF-SOL-013 carries a first-class ID through normalization and serialization. /
 * 有 origin 的 ID 可在现有模型流水线内复用；在 OSPF-SOL-013 将一等 ID 贯穿规范化和序列化前，
 * fallback 明确只保证模型内有效。
 *
 * @return 变量稳定标识 / Stable variable identifier
 */
fun Variable.diagnosticVariableId(): VariableId {
    return id ?: VariableId(
        origin?.let { "${it.identifier}:${it.index}" } ?: "model-local-variable:$index"
    )
}

/**
 * Resolve a linear-row identity for native diagnostics. /
 * 解析原生诊断使用的线性行身份。
 *
 * Row indices are retained only as a model-local disambiguator. They must not be presented as the
 * final cross-rebuild identity before OSPF-SOL-013 is complete. /
 * 行号只作为模型内消歧信息；在 OSPF-SOL-013 完成前不得将其当作跨重建最终身份。
 *
 * @param index 模型约束行索引 / Model constraint row index
 * @return 线性约束稳定标识 / Stable linear constraint identifier
 */
fun LinearTriadModelView.diagnosticConstraintId(index: Int): ConstraintId {
    constraints.ids.getOrNull(index)?.let { return it }
    val originName = constraints.origins.getOrNull(index)
        ?.name
        ?.takeIf { it.isNotBlank() }
    val displayName = constraints.names.getOrNull(index)?.takeIf { it.isNotBlank() }
    val prefix = originName?.let { "origin-constraint:$it" }
        ?: displayName?.let { "model-local-constraint:$it" }
        ?: "model-local-constraint"
    return ConstraintId("$prefix:$index")
}

/**
 * Resolve a quadratic-row identity for native diagnostics. / 解析原生诊断使用的二次行身份。
 *
 * @param index 模型约束行索引 / Model constraint row index
 * @return 二次约束稳定标识 / Stable quadratic constraint identifier
 */
fun QuadraticTetradModelView.diagnosticConstraintId(index: Int): ConstraintId {
    constraints.ids.getOrNull(index)?.let { return it }
    val originName = constraints.origins.getOrNull(index)
        ?.name
        ?.takeIf { it.isNotBlank() }
    val displayName = constraints.names.getOrNull(index)?.takeIf { it.isNotBlank() }
    val prefix = originName?.let { "origin-constraint:$it" }
        ?: displayName?.let { "model-local-constraint:$it" }
        ?: "model-local-constraint"
    return ConstraintId("$prefix:$index")
}
