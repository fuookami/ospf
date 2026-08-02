package fuookami.ospf.kotlin.core.solver.report

import fuookami.ospf.kotlin.core.model.basic.ConstraintRelation as ModelConstraintRelation
import fuookami.ospf.kotlin.core.model.intermediate.LinearTriadModelView
import fuookami.ospf.kotlin.core.model.intermediate.QuadraticTetradModelView
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.operator.*
import fuookami.ospf.kotlin.utils.error.ErrorCode
import fuookami.ospf.kotlin.utils.functional.Failed
import fuookami.ospf.kotlin.utils.functional.Ret
import fuookami.ospf.kotlin.utils.functional.ok

/**
 * Evaluate all linear constraints against a solver-indexed value vector. /
 * 使用求解器索引值向量求值全部线性约束。
 *
 * The result is pure model-side evidence and does not depend on a backend. Missing values or a
 * negative tolerance are returned as structured input errors. /
 * 结果是纯模型侧证据，不依赖后端；缺失变量值或负容差会以结构化输入错误返回。
 *
 * @receiver linear triad model / 线性三元模型
 * @param values solver-indexed variable values / 求解器索引的变量值
 * @param tolerance accepted violation tolerance / 可接受的违反容差
 * @return one evaluation per model constraint / 每个模型约束对应一条求值结果
 */
fun LinearTriadModelView.evaluateConstraints(
    values: List<Flt64>,
    tolerance: Flt64 = Flt64.decimalPrecision
): Ret<List<ConstraintEvaluation<Flt64>>> {
    if (tolerance ls Flt64.zero) {
        return Failed(ErrorCode.IllegalArgument, "约束求值容差不能为负 / Constraint evaluation tolerance must not be negative")
    }
    val evaluations = constraints.indices.map { row ->
        var lhs = Flt64.zero
        for (cell in constraints.lhs[row]) {
            val value = values.getOrNull(cell.colIndex)
                ?: return Failed<List<ConstraintEvaluation<Flt64>>>(
                    ErrorCode.IllegalArgument,
                    "缺少变量值：${cell.colIndex} / Missing value for variable index ${cell.colIndex}"
                )
            lhs += cell.coefficient * value
        }
        val rhs = constraints.rhs[row]
        val relation = constraints.signs[row].toReportRelation()
        val (slack, violation) = relation.slackAndViolation(lhs, rhs)
        ConstraintEvaluation(
            constraintId = diagnosticConstraintId(row),
            lhs = lhs,
            rhs = rhs,
            relation = relation,
            slack = slack,
            violation = violation,
            tolerance = tolerance,
            satisfied = violation leq tolerance
        )
    }
    return ok(evaluations)
}

/**
 * Evaluate all quadratic constraints against a solver-indexed value vector. /
 * 使用求解器索引值向量求值全部二次约束。
 *
 * @receiver quadratic tetrad model / 二次四元模型
 * @param values solver-indexed variable values / 求解器索引的变量值
 * @param tolerance accepted violation tolerance / 可接受的违反容差
 * @return one evaluation per model constraint / 每个模型约束对应一条求值结果
 */
fun QuadraticTetradModelView.evaluateConstraints(
    values: List<Flt64>,
    tolerance: Flt64 = Flt64.decimalPrecision
): Ret<List<ConstraintEvaluation<Flt64>>> {
    if (tolerance ls Flt64.zero) {
        return Failed(ErrorCode.IllegalArgument, "约束求值容差不能为负 / Constraint evaluation tolerance must not be negative")
    }
    val evaluations = constraints.indices.map { row ->
        var lhs = Flt64.zero
        for (cell in constraints.lhs[row]) {
            val first = values.getOrNull(cell.colIndex1)
                ?: return Failed<List<ConstraintEvaluation<Flt64>>>(
                    ErrorCode.IllegalArgument,
                    "缺少变量值：${cell.colIndex1} / Missing value for variable index ${cell.colIndex1}"
                )
            lhs += if (cell.colIndex2 == null) {
                cell.coefficient * first
            } else {
                val second = values.getOrNull(cell.colIndex2)
                    ?: return Failed<List<ConstraintEvaluation<Flt64>>>(
                        ErrorCode.IllegalArgument,
                        "缺少变量值：${cell.colIndex2} / Missing value for variable index ${cell.colIndex2}"
                    )
                cell.coefficient * first * second
            }
        }
        val rhs = constraints.rhs[row]
        val relation = constraints.signs[row].toReportRelation()
        val (slack, violation) = relation.slackAndViolation(lhs, rhs)
        ConstraintEvaluation(
            constraintId = diagnosticConstraintId(row),
            lhs = lhs,
            rhs = rhs,
            relation = relation,
            slack = slack,
            violation = violation,
            tolerance = tolerance,
            satisfied = violation leq tolerance
        )
    }
    return ok(evaluations)
}

private fun ModelConstraintRelation.toReportRelation(): ConstraintRelation = when (this) {
    ModelConstraintRelation.LessEqual -> ConstraintRelation.LessEqual
    ModelConstraintRelation.Equal -> ConstraintRelation.Equal
    ModelConstraintRelation.GreaterEqual -> ConstraintRelation.GreaterEqual
}

private fun ConstraintRelation.slackAndViolation(lhs: Flt64, rhs: Flt64): Pair<Flt64, Flt64> {
    val slack = when (this) {
        ConstraintRelation.LessEqual -> rhs - lhs
        ConstraintRelation.Equal -> rhs - lhs
        ConstraintRelation.GreaterEqual -> lhs - rhs
        ConstraintRelation.Range -> Flt64.zero
    }
    val violation = when (this) {
        ConstraintRelation.Equal -> (lhs - rhs).abs()
        ConstraintRelation.LessEqual, ConstraintRelation.GreaterEqual ->
            if (slack ls Flt64.zero) -slack else Flt64.zero
        ConstraintRelation.Range -> Flt64.zero
    }
    return slack to violation
}
