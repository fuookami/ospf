package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.service

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.physics.quantity.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.*
import fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model.*

data class SolutionAnalyzer(
    private val aggregation: Aggregation
) {
    operator fun invoke(
        solution: List<Flt64>,
        model: AbstractLinearMetaModel
    ): Ret<Solution> {
        val stowage = HashMap<Position, MutableList<Item>>()
        for ((i, item) in aggregation.items.withIndex()) {
            for ((j, position) in aggregation.positions.withIndex()) {
                val xi = aggregation.stowage.x[i, j]
                val token = model.tokens.tokenList.find(xi)
                if (token != null && solution[model.tokens.indexOf(token)] gr Flt64.zero) {
                    stowage.getOrPut(position) { ArrayList() }.add(item)
                }
            }
        }

        val predicateLoadWeight = HashMap<Position, Quantity<Flt64>>()
        for ((j, position) in aggregation.positions.withIndex()) {
            if (position.status.predicateWeightNeeded) {
                val yi = aggregation.load.y[j]
                val token = model.tokens.tokenList.find(yi.value)
                if (token != null && solution[model.tokens.indexOf(token)] gr Flt64.zero) {
                    predicateLoadWeight[position] = Quantity(
                        solution[model.tokens.indexOf(token)],
                        yi.unit
                    ).to(aggregation.aircraftModel.weightUnit)!!
                }
            }
        }

        val recommendedLoadWeight = HashMap<Position, Quantity<Flt64>>()
        for ((j, position) in aggregation.positions.withIndex()) {
            if (position.status.recommendedWeightNeeded) {
                val zi = aggregation.load.z[j]
                val token = model.tokens.tokenList.find(zi.value)
                if (token != null && solution[model.tokens.indexOf(token)] gr Flt64.zero) {
                    recommendedLoadWeight[position] = Quantity(
                        solution[model.tokens.indexOf(token)],
                        zi.unit
                    ).to(aggregation.aircraftModel.weightUnit)!!
                }
            }
        }

        return Ok(
            Solution(
                stowage = stowage,
                predicateLoadWeight = predicateLoadWeight,
                recommendedLoadWeight = recommendedLoadWeight
            )
        )
    }
}
