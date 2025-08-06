package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.soft_security.model

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.utils.multi_array.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*

class DivideEmptyLoading(
    private val positions: List<Position>,
    internal val adjacentPositions: List<PositionPair>,
    private val load: Load
) {
    companion object {
        operator fun invoke(
            positions: List<Position>,
            load: Load
        ): DivideEmptyLoading {
            TODO("not implemented yet")
        }
    }

    lateinit var emptyBetweenCargo: LinearIntermediateSymbols1
    lateinit var emptyCargoBetweenCargo: LinearIntermediateSymbols1
    lateinit var emptyBetweenEmptyCargo: LinearIntermediateSymbols1

    fun register(
        model: AbstractLinearMetaModel
    ): Try {
        if (!::emptyBetweenCargo.isInitialized) {
            emptyBetweenCargo = LinearIntermediateSymbols1("empty_between_cargo", Shape1(adjacentPositions.size)) { p, _ ->
                val position1 = adjacentPositions[p].first
                val position2 = adjacentPositions[p].second
                val j2 = positions.indexOf(position2)
                val loadAmount1 = load.loadAmountOf(position1) { item ->
                    !item.cargo.contains(CargoCode.Empty)
                }
                val loadAmount2 = load.loadAmount[j2]
                if (loadAmount1.range.fixedValue?.let { it eq Flt64.zero } == true) {
                    LinearExpressionSymbol(
                        Flt64.zero,
                        name = "empty_between_cargo_${position1}_${position2}"
                    )
                } else if (position2.status.stowageNeeded || position2.status.adjustmentNeeded) {
                    IfFunction(
                        loadAmount1 geq (loadAmount2 + UInt64.one),
                        name = "empty_between_cargo_${position1}_${position2}"
                    )
                } else if (loadAmount2.range.fixedValue?.let { it eq Flt64.zero } == true) {
                    LinearExpressionSymbol(
                        loadAmount1,
                        name = "empty_between_cargo_${position1}_${position2}"
                    )
                } else {
                    LinearExpressionSymbol(
                        Flt64.zero,
                        name = "empty_between_cargo_${position1}_${position2}"
                    )
                }
            }
        }
        when (val result = model.add(emptyBetweenCargo)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        if (!::emptyCargoBetweenCargo.isInitialized) {
            emptyCargoBetweenCargo = LinearIntermediateSymbols1("empty_cargo_between_cargo", Shape1(adjacentPositions.size)) { p, _ ->
                val position1 = adjacentPositions[p].first
                val position2 = adjacentPositions[p].second

                val loadAmount1 = load.loadAmountOf(position1) { item ->
                    item.cargo.contains(CargoCode.Empty)
                }
                val loadAmount2 = load.loadAmountOf(position2) { item ->
                    !item.cargo.contains(CargoCode.Empty)
                }

                if (loadAmount1.range.fixedValue?.let { it eq Flt64.zero } == true) {
                    LinearExpressionSymbol(
                        Flt64.zero,
                        name = "empty_cargo_between_cargo_${position1}_${position2}"
                    )
                } else if (position2.status.stowageNeeded || position2.status.adjustmentNeeded) {
                    IfFunction(
                        (loadAmount1 + loadAmount2) geq UInt64.two,
                        name = "empty_cargo_between_cargo_${position1}_${position2}"
                    )
                } else if (loadAmount2.range.fixedValue?.let { it eq Flt64.zero } == true) {
                    LinearExpressionSymbol(
                        loadAmount1,
                        name = "empty_cargo_between_cargo_${position1}_${position2}"
                    )
                } else {
                    LinearExpressionSymbol(
                        Flt64.zero,
                        name = "empty_cargo_between_cargo_${position1}_${position2}"
                    )
                }
            }
        }
        when (val result = model.add(emptyCargoBetweenCargo)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        if (!::emptyBetweenEmptyCargo.isInitialized) {
            emptyBetweenEmptyCargo = LinearIntermediateSymbols1("empty_between_empty_cargo", Shape1(adjacentPositions.size)) { p, _ ->
                val position1 = adjacentPositions[p].first
                val j1 = positions.indexOf(position1)
                val position2 = adjacentPositions[p].second
                val j2 = positions.indexOf(position2)
                val loadAmount1 = load.loadAmountOf(position1) { item ->
                    item.cargo.contains(CargoCode.Empty)
                }
                val loadAmount2 = load.loadAmount[j2]
                if (loadAmount1.range.fixedValue?.let { it eq Flt64.zero } == true) {
                    LinearExpressionSymbol(
                        Flt64.zero,
                        name = "empty_between_empty_cargo_${position1}_${position2}"
                    )
                } else if (position2.status.stowageNeeded || position2.status.adjustmentNeeded) {
                    IfFunction(
                        loadAmount1 geq (loadAmount2 + UInt64.one),
                        name = "empty_between_empty_cargo_${position1}_${position2}"
                    )
                } else if (loadAmount2.range.fixedValue?.let { it eq Flt64.zero } == true) {
                    LinearExpressionSymbol(
                        loadAmount1,
                        name = "empty_between_empty_cargo_${position1}_${position2}"
                    )
                } else {
                    LinearExpressionSymbol(
                        Flt64.zero,
                        name = "empty_between_empty_cargo_${position1}_${position2}"
                    )
                }
            }
        }
        when (val result = model.add(emptyBetweenEmptyCargo)) {
            is Ok -> {}

            is Failed -> {
                return Failed(result.error)
            }
        }

        return ok
    }
}
