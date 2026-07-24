package fuookami.ospf.kotlin.core.solver.report

/** 规范化变量 / Normalized variable */
data class NormalizedVariable(
    val id: VariableId,
    val type: String,
    val lowerBound: String? = null,
    val upperBound: String? = null
)

/** 规范化线性项 / Normalized linear term */
data class NormalizedLinearTerm(
    val variableId: VariableId,
    val coefficient: String
)

/** 规范化二次项 / Normalized quadratic term */
data class NormalizedQuadraticTerm(
    val firstVariableId: VariableId,
    val secondVariableId: VariableId,
    val coefficient: String
)

/** 规范化约束 / Normalized constraint */
data class NormalizedConstraint(
    val id: ConstraintId,
    val relation: ConstraintRelation,
    val rhs: String,
    val linearTerms: List<NormalizedLinearTerm>,
    val quadraticTerms: List<NormalizedQuadraticTerm> = emptyList()
)

/** 规范化目标 / Normalized objective */
data class NormalizedObjective(
    val id: ObjectiveId,
    val category: String,
    val constant: String,
    val linearTerms: List<NormalizedLinearTerm>,
    val quadraticTerms: List<NormalizedQuadraticTerm> = emptyList()
)

/**
 * 带版本的规范化数学模型。 / Versioned normalized mathematical model.
 *
 * 数值字段必须由 adapter 使用确定的十进制或特殊值编码，不接受 JVM 对象字符串。 / Numeric fields must use deterministic decimal or special-value encoding supplied by the adapter.
 */
data class NormalizedMathematicalModel(
    val schemaVersion: String = "1.0",
    val modelType: SolverModelType,
    val variables: List<NormalizedVariable>,
    val constraints: List<NormalizedConstraint>,
    val objective: NormalizedObjective
) {
    /** 生成确定性规范文本 / Produce deterministic canonical text */
    fun canonicalText(): String {
        val variableLines = variables.sortedBy { it.id.value }.map { variable ->
            listOf(
                "v",
                encode(variable.id.value),
                encode(variable.type),
                encode(variable.lowerBound ?: ""),
                encode(variable.upperBound ?: "")
            ).joinToString("|")
        }
        val constraintLines = constraints.sortedBy { it.id.value }.map { constraint ->
            val linear = constraint.linearTerms.sortedBy { it.variableId.value }.joinToString(",") { term ->
                "${encode(term.variableId.value)}:${encode(term.coefficient)}"
            }
            val quadratic = constraint.quadraticTerms.sortedWith(
                compareBy({ it.firstVariableId.value }, { it.secondVariableId.value })
            ).joinToString(",") { term ->
                "${encode(term.firstVariableId.value)}:${encode(term.secondVariableId.value)}:${encode(term.coefficient)}"
            }
            listOf(
                "c",
                encode(constraint.id.value),
                constraint.relation.name,
                encode(constraint.rhs),
                linear,
                quadratic
            ).joinToString("|")
        }
        val objectiveLinear = objective.linearTerms.sortedBy { it.variableId.value }.joinToString(",") { term ->
            "${encode(term.variableId.value)}:${encode(term.coefficient)}"
        }
        val objectiveQuadratic = objective.quadraticTerms.sortedWith(
            compareBy({ it.firstVariableId.value }, { it.secondVariableId.value })
        ).joinToString(",") { term ->
            "${encode(term.firstVariableId.value)}:${encode(term.secondVariableId.value)}:${encode(term.coefficient)}"
        }
        val objectiveLine = listOf(
            "o",
            encode(objective.id.value),
            encode(objective.category),
            encode(objective.constant),
            objectiveLinear,
            objectiveQuadratic
        ).joinToString("|")
        return buildList {
            add("schema|${encode(schemaVersion)}")
            add("type|${modelType.name}")
            addAll(variableLines)
            addAll(constraintLines)
            add(objectiveLine)
        }.joinToString("\n")
    }

    /** 生成模型审计指纹 / Produce the model audit fingerprint */
    fun fingerprint(): AuditFingerprint {
        return SolveFingerprinting.sha256(canonicalText(), schemaVersion)
    }

    private fun encode(value: String): String {
        return value.replace("\\", "\\\\").replace("\n", "\\n").replace("|", "\\|")
    }
}
