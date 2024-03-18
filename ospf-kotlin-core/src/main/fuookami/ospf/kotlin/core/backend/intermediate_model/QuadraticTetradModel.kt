package fuookami.ospf.kotlin.core.backend.intermediate_model

import fuookami.ospf.kotlin.core.backend.intermediate_model.QuadraticConstraint
import java.io.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.concept.*
import fuookami.ospf.kotlin.utils.operator.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope

data class QuadraticConstraintCell(
    override val rowIndex: Int,
    val colIndex1: Int,
    val colIndex2: Int?,
    override val coefficient: Flt64
) : ConstraintCell, Cloneable, Copyable<QuadraticConstraintCell> {
    override fun copy() = QuadraticConstraintCell(rowIndex, colIndex1, colIndex2, coefficient.copy())
    override fun clone() = copy()
}

typealias QuadraticConstraint = Constraint<QuadraticConstraintCell>

data class QuadraticObjectiveCell(
    val colIndex1: Int,
    val colIndex2: Int?,
    override val coefficient: Flt64
) : Cell, Cloneable, Copyable<QuadraticObjectiveCell> {
    override fun copy() = QuadraticObjectiveCell(colIndex1, colIndex2, coefficient.copy())
    override fun clone() = copy()
}

typealias QuadraticObjective = Objective<QuadraticObjectiveCell>

class BasicQuadraticTetradModel(
    override val variables: List<Variable>,
    override val constraints: QuadraticConstraint,
    override val name: String
) : BasicModelView<QuadraticConstraintCell>, Cloneable, Copyable<BasicQuadraticTetradModel> {
    override fun copy() = BasicQuadraticTetradModel(
        variables.map { it.copy() },
        constraints.copy(),
        name
    )

    override fun clone() = copy()

    override fun exportLP(writer: FileWriter): Try {
        writer.append("Subject To\n")
        var i = 0
        var j = 0
        while (i != constraints.size) {
            writer.append(" ${constraints.names[i]}: ")
            var flag = false
            var k = 0
            while (j != constraints.lhs.size && i == constraints.lhs[j].rowIndex) {
                val coefficient = if (k != 0) {
                    if (constraints.lhs[j].coefficient leq Flt64.zero) {
                        writer.append(" - ")
                    } else {
                        writer.append(" + ")
                    }
                    abs(constraints.lhs[j].coefficient)
                } else {
                    constraints.lhs[j].coefficient
                }
                if (coefficient neq Flt64.zero) {
                    if (coefficient neq Flt64.one) {
                        writer.append("$coefficient ")
                    }
                    if (constraints.lhs[j].colIndex2 == null) {
                        writer.append("${variables[constraints.lhs[j].colIndex1]}")
                    } else {
                        writer.append("${variables[constraints.lhs[j].colIndex1]} * ${variables[constraints.lhs[j].colIndex2!!]}")
                    }
                }
                flag = true
                ++j
                ++k
            }
            if (!flag) {
                writer.append("0")
            }
            writer.append(" ${constraints.signs[i]} ${constraints.rhs[i]}\n")
            ++i
        }
        writer.append("\n")

        writer.append("Bounds\n")
        for (variable in variables) {
            val lowerInf = variable.lowerBound.isNegativeInfinity()
            val upperInf = variable.upperBound.isInfinity()
            if (lowerInf && upperInf) {
                writer.append(" $variable free\n")
            } else if (lowerInf) {
                writer.append(" $variable <= ${variable.upperBound}\n")
            } else if (upperInf) {
                writer.append(" $variable >= ${variable.lowerBound}\n")
            } else {
                if (variable.lowerBound eq variable.upperBound) {
                    writer.append(" $variable = ${variable.lowerBound}\n")
                } else {
                    writer.append(" ${variable.lowerBound} <= $variable <= ${variable.upperBound}\n")
                }
            }
        }
        writer.append("\n")

        if (containsBinary) {
            writer.append("Binaries\n")
            for (variable in variables) {
                if (variable.type.isBinaryType) {
                    writer.append(" $variable")
                }
            }
            writer.append("\n")
        }

        if (containsNotBinaryInteger) {
            writer.append("Generals\n")
            for (variable in variables) {
                if (variable.type.isNotBinaryIntegerType) {
                    writer.append(" $variable")
                }
            }
            writer.append("\n")
        }

        writer.append("End\n")
        return ok
    }
}

typealias QuadraticTetradModelView = ModelView<QuadraticConstraintCell, QuadraticObjectiveCell>

data class QuadraticTetradModel(
    private val impl: BasicQuadraticTetradModel,
    override val objective: QuadraticObjective,
) : QuadraticTetradModelView, Cloneable, Copyable<QuadraticTetradModel> {
    override val variables: List<Variable> by impl::variables
    override val constraints: QuadraticConstraint by impl::constraints
    override val name: String by impl::name

    companion object {
        suspend operator fun invoke(model: QuadraticModel): QuadraticTetradModel {
            val tokens = model.tokens.tokens
            val tokenIndexes = model.tokens.tokenIndexMap

            return coroutineScope {
                val variablePromise = async(Dispatchers.Default) {
                    val variables = ArrayList<Variable?>()
                    for (i in tokens.indices) {
                        variables.add(null)
                    }
                    for (token in tokens) {
                        val index = tokenIndexes[token]!!
                        variables[index] = Variable(
                            index,
                            token.lowerBound,
                            token.upperBound,
                            token.variable.type,
                            token.variable.name,
                            token.result
                        )
                    }
                    variables.map { it!! }
                }

                val constraintPromise = async(Dispatchers.Default) {
                    val lhs = ArrayList<QuadraticConstraintCell>()
                    val signs = ArrayList<Sign>()
                    val rhs = ArrayList<Flt64>()
                    val names = ArrayList<String>()
                    for ((index, constraint) in model.constraints.withIndex()) {
                        for (cell in constraint.lhs) {
                            val temp = cell as QuadraticCell
                            lhs.add(
                                QuadraticConstraintCell(
                                    index,
                                    tokenIndexes[temp.token1]!!,
                                    temp.token2?.let { tokenIndexes[it]!! },
                                    temp.coefficient
                                )
                            )
                        }
                        signs.add(constraint.sign)
                        rhs.add(constraint.rhs)
                        names.add(constraint.name)
                    }
                    QuadraticConstraint(lhs, signs, rhs, names)
                }

                val objectiveCategory = if (model.objectFunction.subObjects.size == 1) {
                    model.objectFunction.subObjects.first().category
                } else {
                    model.objectFunction.category
                }

                val objectivePromise = async(Dispatchers.Default) {
                    val coefficient = tokens.indices.map { HashMap<Int?, Flt64>() }.toMutableList()
                    for (subObject in model.objectFunction.subObjects) {
                        if (subObject.category == objectiveCategory) {
                            for (cell in subObject.cells) {
                                val temp = cell as QuadraticCell
                                val value = coefficient[tokenIndexes[temp.token1]!!][temp.token2?.let { tokenIndexes[it]!! }] ?: Flt64.zero
                                coefficient[tokenIndexes[temp.token1]!!][temp.token2?.let { tokenIndexes[it]!! }] = value + temp.coefficient
                            }
                        } else {
                            for (cell in subObject.cells) {
                                val temp = cell as QuadraticCell
                                val value = coefficient[tokenIndexes[temp.token1]!!][temp.token2?.let { tokenIndexes[it]!! }] ?: Flt64.zero
                                coefficient[tokenIndexes[temp.token1]!!][temp.token2?.let { tokenIndexes[it]!! }] = value - temp.coefficient
                            }
                        }
                    }

                    val objective = ArrayList<QuadraticObjectiveCell>()
                    for (i in tokens.indices) {
                        for ((j, value) in coefficient[i]) {
                            objective.add(
                                QuadraticObjectiveCell(
                                    i,
                                    j,
                                    value
                                )
                            )
                        }
                    }

                    objective
                }

                QuadraticTetradModel(
                    BasicQuadraticTetradModel(
                        variablePromise.await(),
                        constraintPromise.await(),
                        model.name
                    ),
                    QuadraticObjective(objectiveCategory, objectivePromise.await())
                )
            }
        }
    }

    override fun copy() = QuadraticTetradModel(impl.copy(), objective.copy())
    override fun clone() = copy()

    override fun exportLP(writer: FileWriter): Try {
        writer.write("${objective.category}\n")
        var i = 0
        for (cell in objective.obj) {
            if (cell.coefficient eq Flt64.zero) {
                continue
            }
            val coefficient = if (i != 0) {
                if (cell.coefficient leq Flt64.zero) {
                    writer.append(" - ")
                } else {
                    writer.append(" + ")
                }
                abs(cell.coefficient)
            } else {
                cell.coefficient
            }
            if (coefficient neq Flt64.zero) {
                if (coefficient neq Flt64.one) {
                    writer.append("$coefficient ")
                }
                writer.append("${variables[cell.colIndex1]}")

                if (cell.colIndex2 != null) {
                    writer.append(" * ${variables[cell.colIndex2]}")
                }
            }
            ++i
        }
        writer.append("\n\n")

        return when (val result = impl.exportLP(writer)) {
            is Failed -> {
                Failed(result.error)
            }

            is Ok -> {
                ok
            }
        }
    }
}
