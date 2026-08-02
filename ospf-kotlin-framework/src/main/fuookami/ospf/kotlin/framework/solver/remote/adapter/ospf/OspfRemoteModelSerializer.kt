/**
 * OSPF 远程模型序列化器 / OSPF remote model serializer
*/
package fuookami.ospf.kotlin.framework.solver.remote.adapter.ospf

import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.model.basic.*
import fuookami.ospf.kotlin.core.model.intermediate.*
import fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingModelSnapshot
import fuookami.ospf.kotlin.core.model.constraint_programming.ConstraintProgrammingSnapshotCodec
import fuookami.ospf.kotlin.framework.solver.remote.domain.*

/**
 * OSPF 远程模型序列化器。 / OSPF remote model serializer.
*/
object OspfRemoteModelSerializer {

    /**
     * 序列化线性三元模型。 / Serialize linear triad model.
     *
     * @param model 线性三元模型视图 / Linear triad model view
     * @return 序列化线性模型 / Serialized linear model
    */
    fun serialize(model: LinearTriadModelView): SerializedLinearModel {
        return SerializedLinearModel(
            name = model.name,
            variables = model.variables.map { it.toSerializedVariable() },
            constraints = model.constraints.indices.map { rowIndex ->
                SerializedConstraint(
                    cells = model.constraints.lhs[rowIndex].map { cell ->
                        SerializedConstraintCell(
                            rowIndex = cell.rowIndex,
                            colIndex = cell.colIndex,
                            coefficient = cell.coefficient
                        )
                    },
                    sign = model.constraints.signs[rowIndex].toSerializedSign(),
                    rhs = model.constraints.rhs[rowIndex],
                    name = model.constraints.names[rowIndex].ifEmpty { "cons$rowIndex" },
                    identityId = model.constraints.ids.getOrNull(rowIndex)?.value,
                    identityScope = model.constraints.identityScopeAt(rowIndex).name.uppercase(),
                    identityOriginKind = model.constraints.identityOriginAt(rowIndex)?.kind,
                    identityOriginKey = model.constraints.identityOriginAt(rowIndex)?.key,
                    identityNamespace = model.constraints.identityNamespace,
                    identitySchemaVersion = model.constraints.identitySchemaVersion
                )
            },
            objective = SerializedObjective(
                category = model.objective.category.toSerializedCategory(),
                cells = model.objective.objective.map { cell ->
                    SerializedObjectiveCell(
                        colIndex = cell.colIndex,
                        coefficient = cell.coefficient
                    )
                },
                constant = model.objective.constant,
                identityId = model.objective.id?.value,
                identityScope = model.objective.identityScope.name.uppercase(),
                identityOriginKind = model.objective.identityOrigin?.kind,
                identityOriginKey = model.objective.identityOrigin?.key,
                identityNamespace = model.objective.identityNamespace,
                identitySchemaVersion = model.objective.identitySchemaVersion
            ),
            identityNamespace = model.identityNamespace,
            identitySchemaVersion = model.identitySchemaVersion
        )
    }

    /**
     * 序列化二次四元模型。 / Serialize quadratic tetrad model.
     *
     * @param model 二次四元模型视图 / Quadratic tetrad model view
     * @return 序列化二次模型 / Serialized quadratic model
    */
    fun serialize(model: QuadraticTetradModelView): SerializedQuadraticModel {
        return SerializedQuadraticModel(
            name = model.name,
            variables = model.variables.map { it.toSerializedVariable() },
            linearConstraints = model.constraints.indices.map { rowIndex ->
                SerializedConstraint(
                    cells = model.constraints.lhs[rowIndex].filter { it.colIndex2 == null }.map { cell ->
                        SerializedConstraintCell(
                            rowIndex = cell.rowIndex,
                            colIndex = cell.colIndex1,
                            coefficient = cell.coefficient
                        )
                    },
                    sign = model.constraints.signs[rowIndex].toSerializedSign(),
                    rhs = model.constraints.rhs[rowIndex],
                    name = model.constraints.names[rowIndex].ifEmpty { "cons$rowIndex" },
                    identityId = model.constraints.ids.getOrNull(rowIndex)?.value,
                    identityScope = model.constraints.identityScopeAt(rowIndex).name.uppercase(),
                    identityOriginKind = model.constraints.identityOriginAt(rowIndex)?.kind,
                    identityOriginKey = model.constraints.identityOriginAt(rowIndex)?.key,
                    identityNamespace = model.constraints.identityNamespace,
                    identitySchemaVersion = model.constraints.identitySchemaVersion
                )
            },
            quadraticConstraints = model.constraints.indices.map { rowIndex ->
                SerializedQuadraticConstraint(
                    linearCells = model.constraints.lhs[rowIndex].filter { it.colIndex2 == null }.map { cell ->
                        SerializedConstraintCell(
                            rowIndex = cell.rowIndex,
                            colIndex = cell.colIndex1,
                            coefficient = cell.coefficient
                        )
                    },
                    quadraticCells = model.constraints.lhs[rowIndex].mapNotNull { cell ->
                        cell.colIndex2?.let { colIndex2 ->
                            SerializedQuadraticConstraintCell(
                                rowIndex = cell.rowIndex,
                                colIndex1 = cell.colIndex1,
                                colIndex2 = colIndex2,
                                coefficient = cell.coefficient
                            )
                        }
                    },
                    sign = model.constraints.signs[rowIndex].toSerializedSign(),
                    rhs = model.constraints.rhs[rowIndex],
                    name = model.constraints.names[rowIndex].ifEmpty { "cons$rowIndex" },
                    identityId = model.constraints.ids.getOrNull(rowIndex)?.value,
                    identityScope = model.constraints.identityScopeAt(rowIndex).name.uppercase(),
                    identityOriginKind = model.constraints.identityOriginAt(rowIndex)?.kind,
                    identityOriginKey = model.constraints.identityOriginAt(rowIndex)?.key,
                    identityNamespace = model.constraints.identityNamespace,
                    identitySchemaVersion = model.constraints.identitySchemaVersion
                )
            },
            objective = SerializedQuadraticObjective(
                category = model.objective.category.toSerializedCategory(),
                linearCells = model.objective.objective.filter { it.colIndex2 == null }.map { cell ->
                    SerializedObjectiveCell(
                        colIndex = cell.colIndex1,
                        coefficient = cell.coefficient
                    )
                },
                quadraticCells = model.objective.objective.mapNotNull { cell ->
                    cell.colIndex2?.let { colIndex2 ->
                        SerializedQuadraticObjectiveCell(
                            colIndex1 = cell.colIndex1,
                            colIndex2 = colIndex2,
                            coefficient = cell.coefficient
                        )
                    }
                },
                constant = model.objective.constant,
                identityId = model.objective.id?.value,
                identityScope = model.objective.identityScope.name.uppercase(),
                identityOriginKind = model.objective.identityOrigin?.kind,
                identityOriginKey = model.objective.identityOrigin?.key,
                identityNamespace = model.objective.identityNamespace,
                identitySchemaVersion = model.objective.identitySchemaVersion
            ),
            identityNamespace = model.identityNamespace,
            identitySchemaVersion = model.identitySchemaVersion
        )
    }

    /**
     * 序列化为模型数据。 / Serialize to model data.
     *
     * @param model 线性三元模型视图 / Linear triad model view
     * @return 远程模型数据 / Remote model data
    */
    fun modelData(model: LinearTriadModelView): ModelData {
        return ModelData.linear(serialize(model))
    }

    /**
     * 序列化为模型数据。 / Serialize to model data.
     *
     * @param model 二次四元模型视图 / Quadratic tetrad model view
     * @return 远程模型数据 / Remote model data
    */
    fun modelData(model: QuadraticTetradModelView): ModelData {
        return ModelData.quadratic(serialize(model))
    }

    /**
     * 序列化 CP snapshot 为原始模型数据。 / Serialize a CP snapshot as raw model data.
     *
     * @param snapshot 不含 native 句柄的 CP snapshot / CP snapshot without native handles
     * @return 远程模型数据或结构化错误 / Remote model data or a structured error
    */
    fun modelData(snapshot: ConstraintProgrammingModelSnapshot): Ret<ModelData> {
        return ConstraintProgrammingSnapshotCodec.encode(snapshot)
            .map { encoded ->
                ModelData.raw(
                    bytes = encoded.encodeToByteArray(),
                    format = "ospf-cp-snapshot-json"
                )
            }
    }
}

/**
 * 转换为远程序列化变量。 / Convert to remote serialized variable.
 *
 * @return 序列化变量 / Serialized variable
*/
fun Variable.toSerializedVariable(): SerializedVariable {
    return SerializedVariable(
        index = index,
        name = name,
        lowerBound = lowerBound,
        upperBound = upperBound,
        type = when {
            type.isBinaryType -> SerializedVariableType.BINARY
            type.isIntegerType -> SerializedVariableType.INTEGER
            else -> SerializedVariableType.CONTINUOUS
        },
        identityId = id?.value,
        identityScope = identityScope.name.uppercase(),
        identityOriginKind = identityOrigin?.kind,
        identityOriginKey = identityOrigin?.key,
        identityNamespace = identityNamespace,
        identitySchemaVersion = identitySchemaVersion
    )
}

/**
 * 转换为远程序列化约束符号。 / Convert to remote serialized constraint sign.
 *
 * @return 序列化约束符号 / Serialized constraint sign
*/
fun ConstraintRelation.toSerializedSign(): SerializedConstraintSign {
    return when (this) {
        ConstraintRelation.LessEqual -> SerializedConstraintSign.LESS_EQUAL
        ConstraintRelation.GreaterEqual -> SerializedConstraintSign.GREATER_EQUAL
        ConstraintRelation.Equal -> SerializedConstraintSign.EQUAL
    }
}

/**
 * 转换为远程序列化目标类型。 / Convert to remote serialized objective category.
 *
 * @return 序列化目标类型 / Serialized objective category
*/
fun ObjectCategory.toSerializedCategory(): SerializedObjectiveCategory {
    return when (this) {
        ObjectCategory.Minimum -> SerializedObjectiveCategory.MINIMIZE
        ObjectCategory.Maximum -> SerializedObjectiveCategory.MAXIMIZE
    }
}
