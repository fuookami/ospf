package fuookami.ospf.kotlin.example.core_demo

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.concept.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.utils.multi_array.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*

/**
 * @see     https://fuookami.github.io/ospf/examples/example14.html
 */
data object Demo14 {
    sealed interface Node : Indexed {
        val name: String
    }

    class Product(
        override val name: String,
        val storage: UInt64
    ) : Node, AutoIndexed(Node::class)

    class Sale(
        override val name: String,
        val demand: UInt64
    ) : Node, AutoIndexed(Node::class)

    class Distribution(
        override val name: String
    ) : Node, AutoIndexed(Node::class)

    val nodes: List<Node> = listOf(
        Product("广州", UInt64(600)),
        Product("大连", UInt64(400)),
        Sale("南京", UInt64(200)),
        Sale("济南", UInt64(150)),
        Sale("南昌", UInt64(350)),
        Sale("青岛", UInt64(300)),
        Distribution("上海"),
        Distribution("天津")
    )

    val unitCost = mapOf(
        nodes[0] to mapOf(
            nodes[6] to UInt64(2),
            nodes[7] to UInt64(3)
        ),
        nodes[1] to mapOf(
            nodes[5] to UInt64(4),
            nodes[6] to UInt64(3),
            nodes[7] to UInt64(1)
        ),
        nodes[6] to mapOf(
            nodes[2] to UInt64(2),
            nodes[3] to UInt64(6),
            nodes[4] to UInt64(3),
            nodes[5] to UInt64(6)
        ),
        nodes[7] to mapOf(
            nodes[2] to UInt64(4),
            nodes[3] to UInt64(4),
            nodes[4] to UInt64(6),
            nodes[5] to UInt64(5)
        )
    )

    lateinit var x: UIntVariable2

    lateinit var cost: LinearIntermediateSymbol
    lateinit var transOut: LinearIntermediateSymbols1
    lateinit var transIn: LinearIntermediateSymbols1

    val metaModel = LinearMetaModel("demo14")

    private val subProcesses = listOf(
        Demo14::initVariable,
        Demo14::initSymbol,
        Demo14::initObject,
        Demo14::initConstraint,
        Demo14::solve,
        Demo14::analyzeSolution
    )

    suspend operator fun invoke(): Try {
        for (process in subProcesses) {
            when (val result = process()) {
                is Ok -> {}

                is Failed -> {
                    return Failed(result.error)
                }
            }
        }
        return ok
    }

    private suspend fun initVariable(): Try {
        x = UIntVariable2("x", Shape2(nodes.size, nodes.size))
        for (node1 in nodes) {
            for (node2 in nodes) {
                val thisUnitCost = unitCost[node1]?.get(node2)
                if (thisUnitCost != null) {
                    if (node1 is Product) {
                        x[node1, node2].range.leq(node1.storage)
                    }
                    metaModel.add(x[node1, node2])
                } else {
                    x[node1, node2].range.eq(UInt64.zero)
                }
            }
        }

        return ok
    }

    private suspend fun initSymbol(): Try {
        cost = LinearExpressionSymbol(
            sum(nodes.flatMap { node1 ->
                nodes.mapNotNull { node2 ->
                    unitCost[node1]?.get(node2)?.let {
                        it * x[node1, node2]
                    }
                }
            }), "cost"
        )
        metaModel.add(cost)

        transOut = LinearIntermediateSymbols1("out", Shape1(nodes.size)) { i, _ ->
            val node = nodes[i]
            LinearExpressionSymbol(sum(x[node, _a]), "out_${node.name}")
        }
        metaModel.add(transOut)

        transIn = LinearIntermediateSymbols1("in", Shape1(nodes.size)) { i, _ ->
            val node = nodes[i]
            LinearExpressionSymbol(sum(x[_a, node]), "in_${node.name}")
        }
        metaModel.add(transIn)

        return ok
    }

    private suspend fun initObject(): Try {
        metaModel.minimize(cost)

        return ok
    }

    private suspend fun initConstraint(): Try {
        for (node in nodes.filterIsInstance<Product>()) {
            metaModel.addConstraint(
                transOut[node] leq node.storage,
                "out_${node.name}"
            )
        }

        for (node in nodes.filterIsInstance<Sale>()) {
            metaModel.addConstraint(
                transIn[node] geq node.demand,
                "in_${node.name}"
            )
        }

        for (node in nodes.filterIsInstance<Distribution>()) {
            metaModel.addConstraint(
                transOut[node] eq transIn[node],
                "balance_${node.name}"
            )
        }

        return ok
    }

    private suspend fun solve(): Try {
        val solver = ScipLinearSolver()
        when (val ret = solver(metaModel)) {
            is Ok -> {
                metaModel.tokens.setSolution(ret.value.solution)
            }

            is Failed -> {
                return Failed(ret.error)
            }
        }

        return ok
    }

    private suspend fun analyzeSolution(): Try {
        val trans: MutableMap<Node, MutableMap<Node, UInt64>> = hashMapOf()
        for (token in metaModel.tokens.tokens) {
            if (token.result!! geq Flt64.one && token.variable belongsTo x) {
                val vector = token.variable.vectorView
                val from = nodes[vector[0]]
                val to = nodes[vector[1]]
                trans.getOrPut(from) { hashMapOf() }[to] = token.result!!.round().toUInt64()
            }
        }

        return ok
    }
}
