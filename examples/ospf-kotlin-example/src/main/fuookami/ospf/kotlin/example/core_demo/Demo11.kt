package fuookami.ospf.kotlin.example.core_demo

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.concept.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.utils.multi_array.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*

data object Demo11 {
    sealed class Node : AutoIndexed(Node::class)
    class RootNode : Node()
    class EndNode : Node()
    class NormalNode : Node()

    val nodes: List<Node> = listOf(
        listOf(RootNode()),
        (1..7).map { NormalNode() },
        listOf(EndNode())
    ).flatten()

    val capacities = mapOf(
        nodes[0] to mapOf(
            nodes[1] to UInt64(15),
            nodes[2] to UInt64(10),
            nodes[3] to UInt64(40)
        ),
        nodes[1] to mapOf(
            nodes[4] to UInt64(15)
        ),
        nodes[2] to mapOf(
            nodes[5] to UInt64(10),
            nodes[6] to UInt64(35)
        ),
        nodes[3] to mapOf(
            nodes[6] to UInt64(30),
            nodes[7] to UInt64(20)
        ),
        nodes[4] to mapOf(
            nodes[6] to UInt64(10)
        ),
        nodes[5] to mapOf(
            nodes[8] to UInt64(10)
        ),
        nodes[6] to mapOf(
            nodes[7] to UInt64(10)
        ),
        nodes[7] to mapOf(
            nodes[8] to UInt64(45)
        )
    )

    lateinit var x: UIntVariable2
    lateinit var flow: UIntVar

    lateinit var flowIn: LinearSymbols1
    lateinit var flowOut: LinearSymbols1

    val metaModel = LinearMetaModel("demo11")

    private val subProcesses = listOf(
        Demo11::initVariable,
        Demo11::initSymbol,
        Demo11::initObject,
        Demo11::initConstraint,
        Demo11::solve,
        Demo11::analyzeSolution
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
                if (node1 == node2) {
                    continue
                }
                capacities[node1]?.get(node2)?.let {
                    val xi = x[node1, node2]
                    xi.range.leq(it)
                    metaModel.add(xi)
                }
            }
        }
        flow = UIntVar("flow")
        metaModel.add(flow)
        return ok
    }

    private suspend fun initSymbol(): Try {
        flowIn = LinearSymbols1("flow_in", Shape1(nodes.size)) { i, _ ->
            LinearExpressionSymbol(sum(x[_a, i]), "flow_in_$i")
        }
        metaModel.add(flowIn)
        flowOut = LinearSymbols1("flow_out", Shape1(nodes.size)) { i, _ ->
            LinearExpressionSymbol(sum(x[i, _a]), "flow_out_$i")
        }
        metaModel.add(flowOut)
        return ok
    }

    private suspend fun initObject(): Try {
        metaModel.maximize(flow, "flow")
        return ok
    }

    private suspend fun initConstraint(): Try {
        val rootNode = nodes.first { it is RootNode }
        metaModel.addConstraint(
            flowOut[rootNode] - flowIn[rootNode] eq flow,
            "flow_${rootNode.index}"
        )
        val endNode = nodes.first { it is EndNode }
        metaModel.addConstraint(
            flowIn[endNode] - flowOut[endNode] eq flow,
            "flow_${endNode.index}"
        )
        for (node in nodes.filterIsInstance<NormalNode>()) {
            metaModel.addConstraint(
                flowOut[node] eq flowIn[node],
                "flow_${node.index}"
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
        val flow: MutableMap<Node, MutableMap<Node, UInt64>> = hashMapOf()
        for (token in metaModel.tokens.tokens) {
            if (token.result!! geq Flt64.one && token.variable belongsTo x) {
                val vector = token.variable.vectorView
                val node1 = nodes[vector[0]]
                val node2 = nodes[vector[1]]
                flow.getOrPut(node1) { hashMapOf() }[node2] = token.result!!.round().toUInt64()
            }
        }
        return ok
    }
}
