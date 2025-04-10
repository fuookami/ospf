package fuookami.ospf.kotlin.example.core_demo

import fuookami.ospf.kotlin.core.backend.plugins.gurobi.GurobiLinearSolver
import kotlin.time.Duration.Companion.seconds
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.math.geometry.*
import fuookami.ospf.kotlin.utils.math.value_range.*
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
import fuookami.ospf.kotlin.core.backend.solver.config.SolverConfig

data object Demo17 {
    sealed interface Node : Indexed {
        val demand: UInt64 get() = UInt64.zero
        val position: Point2
        val timeWindow: ValueRange<UInt64>

        fun distance(other: Node): Flt64 {
            return position.distance(other.position)
        }

        fun cost(other: Node): Flt64 {
            return distance(other)
        }

        fun time(other: Node): Flt64 {
            return distance(other)
        }
    }

    data class OriginNode(
        override val position: Point2,
        override val timeWindow: ValueRange<UInt64>
    ) : Node, AutoIndexed(Node::class)

    data class EndNode(
        override val position: Point2,
        override val timeWindow: ValueRange<UInt64>
    ) : Node, AutoIndexed(Node::class)

    data class DemandNode(
        override val position: Point2,
        override val timeWindow: ValueRange<UInt64>,
        override val demand: UInt64,
        val serviceTime: UInt64
    ) : Node, AutoIndexed(Node::class)

    data class Vehicle(
        val capacity: UInt64,
        val fixedUsedCost: UInt64,
    ) : AutoIndexed(Vehicle::class)

    val nodes: List<Node> = listOf(
        OriginNode(Point2(Flt64(40), Flt64(50)), ValueRange(UInt64(0), UInt64(1236)).value!!),
        DemandNode(Point2(Flt64(45), Flt64(68)), ValueRange(UInt64(912), UInt64(967)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(45), Flt64(70)), ValueRange(UInt64(825), UInt64(870)).value!!, UInt64(30), UInt64(90)),
        DemandNode(Point2(Flt64(42), Flt64(66)), ValueRange(UInt64(65), UInt64(146)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(42), Flt64(68)), ValueRange(UInt64(727), UInt64(782)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(42), Flt64(65)), ValueRange(UInt64(15), UInt64(67)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(40), Flt64(69)), ValueRange(UInt64(621), UInt64(702)).value!!, UInt64(20), UInt64(90)),
        DemandNode(Point2(Flt64(40), Flt64(66)), ValueRange(UInt64(170), UInt64(225)).value!!, UInt64(20), UInt64(90)),
        DemandNode(Point2(Flt64(38), Flt64(68)), ValueRange(UInt64(255), UInt64(324)).value!!, UInt64(20), UInt64(90)),
        DemandNode(Point2(Flt64(38), Flt64(70)), ValueRange(UInt64(534), UInt64(605)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(35), Flt64(66)), ValueRange(UInt64(357), UInt64(410)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(35), Flt64(69)), ValueRange(UInt64(448), UInt64(505)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(25), Flt64(85)), ValueRange(UInt64(652), UInt64(721)).value!!, UInt64(20), UInt64(90)),
        DemandNode(Point2(Flt64(22), Flt64(75)), ValueRange(UInt64(30), UInt64(92)).value!!, UInt64(30), UInt64(90)),
        DemandNode(Point2(Flt64(22), Flt64(85)), ValueRange(UInt64(567), UInt64(620)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(20), Flt64(80)), ValueRange(UInt64(384), UInt64(429)).value!!, UInt64(40), UInt64(90)),
        DemandNode(Point2(Flt64(20), Flt64(85)), ValueRange(UInt64(475), UInt64(528)).value!!, UInt64(40), UInt64(90)),
        DemandNode(Point2(Flt64(18), Flt64(75)), ValueRange(UInt64(99), UInt64(148)).value!!, UInt64(20), UInt64(90)),
        DemandNode(Point2(Flt64(15), Flt64(75)), ValueRange(UInt64(179), UInt64(254)).value!!, UInt64(20), UInt64(90)),
        DemandNode(Point2(Flt64(15), Flt64(80)), ValueRange(UInt64(278), UInt64(345)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(30), Flt64(50)), ValueRange(UInt64(10), UInt64(73)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(30), Flt64(52)), ValueRange(UInt64(914), UInt64(965)).value!!, UInt64(20), UInt64(90)),
        DemandNode(Point2(Flt64(28), Flt64(52)), ValueRange(UInt64(812), UInt64(883)).value!!, UInt64(20), UInt64(90)),
        DemandNode(Point2(Flt64(28), Flt64(55)), ValueRange(UInt64(732), UInt64(777)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(25), Flt64(50)), ValueRange(UInt64(65), UInt64(144)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(25), Flt64(52)), ValueRange(UInt64(169), UInt64(224)).value!!, UInt64(40), UInt64(90)),
        DemandNode(Point2(Flt64(25), Flt64(55)), ValueRange(UInt64(622), UInt64(701)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(23), Flt64(52)), ValueRange(UInt64(261), UInt64(316)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(23), Flt64(55)), ValueRange(UInt64(546), UInt64(593)).value!!, UInt64(20), UInt64(90)),
        DemandNode(Point2(Flt64(20), Flt64(50)), ValueRange(UInt64(358), UInt64(405)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(20), Flt64(55)), ValueRange(UInt64(449), UInt64(504)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(10), Flt64(35)), ValueRange(UInt64(200), UInt64(237)).value!!, UInt64(20), UInt64(90)),
        DemandNode(Point2(Flt64(10), Flt64(40)), ValueRange(UInt64(31), UInt64(100)).value!!, UInt64(30), UInt64(90)),
        DemandNode(Point2(Flt64(8), Flt64(40)), ValueRange(UInt64(87), UInt64(158)).value!!, UInt64(40), UInt64(90)),
        DemandNode(Point2(Flt64(8), Flt64(45)), ValueRange(UInt64(751), UInt64(816)).value!!, UInt64(20), UInt64(90)),
        DemandNode(Point2(Flt64(5), Flt64(35)), ValueRange(UInt64(283), UInt64(344)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(5), Flt64(45)), ValueRange(UInt64(665), UInt64(716)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(2), Flt64(40)), ValueRange(UInt64(383), UInt64(434)).value!!, UInt64(20), UInt64(90)),
        DemandNode(Point2(Flt64(0), Flt64(40)), ValueRange(UInt64(479), UInt64(522)).value!!, UInt64(30), UInt64(90)),
        DemandNode(Point2(Flt64(0), Flt64(45)), ValueRange(UInt64(567), UInt64(624)).value!!, UInt64(20), UInt64(90)),
        DemandNode(Point2(Flt64(35), Flt64(30)), ValueRange(UInt64(264), UInt64(321)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(35), Flt64(32)), ValueRange(UInt64(166), UInt64(235)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(33), Flt64(32)), ValueRange(UInt64(68), UInt64(149)).value!!, UInt64(20), UInt64(90)),
        DemandNode(Point2(Flt64(33), Flt64(35)), ValueRange(UInt64(16), UInt64(80)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(32), Flt64(30)), ValueRange(UInt64(359), UInt64(412)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(30), Flt64(30)), ValueRange(UInt64(541), UInt64(600)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(30), Flt64(32)), ValueRange(UInt64(448), UInt64(509)).value!!, UInt64(30), UInt64(90)),
        DemandNode(Point2(Flt64(30), Flt64(35)), ValueRange(UInt64(1054), UInt64(1127)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(28), Flt64(30)), ValueRange(UInt64(632), UInt64(693)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(28), Flt64(35)), ValueRange(UInt64(1001), UInt64(1066)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(26), Flt64(32)), ValueRange(UInt64(815), UInt64(880)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(25), Flt64(30)), ValueRange(UInt64(725), UInt64(786)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(25), Flt64(35)), ValueRange(UInt64(912), UInt64(969)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(44), Flt64(5)), ValueRange(UInt64(286), UInt64(347)).value!!, UInt64(20), UInt64(90)),
        DemandNode(Point2(Flt64(42), Flt64(10)), ValueRange(UInt64(186), UInt64(257)).value!!, UInt64(40), UInt64(90)),
        DemandNode(Point2(Flt64(42), Flt64(15)), ValueRange(UInt64(95), UInt64(158)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(40), Flt64(5)), ValueRange(UInt64(385), UInt64(436)).value!!, UInt64(30), UInt64(90)),
        DemandNode(Point2(Flt64(40), Flt64(15)), ValueRange(UInt64(35), UInt64(87)).value!!, UInt64(40), UInt64(90)),
        DemandNode(Point2(Flt64(38), Flt64(5)), ValueRange(UInt64(471), UInt64(534)).value!!, UInt64(30), UInt64(90)),
        DemandNode(Point2(Flt64(38), Flt64(15)), ValueRange(UInt64(651), UInt64(740)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(35), Flt64(5)), ValueRange(UInt64(562), UInt64(629)).value!!, UInt64(20), UInt64(90)),
        DemandNode(Point2(Flt64(50), Flt64(30)), ValueRange(UInt64(531), UInt64(610)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(50), Flt64(35)), ValueRange(UInt64(262), UInt64(317)).value!!, UInt64(20), UInt64(90)),
        DemandNode(Point2(Flt64(50), Flt64(40)), ValueRange(UInt64(171), UInt64(218)).value!!, UInt64(50), UInt64(90)),
        DemandNode(Point2(Flt64(48), Flt64(30)), ValueRange(UInt64(632), UInt64(693)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(48), Flt64(40)), ValueRange(UInt64(76), UInt64(129)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(47), Flt64(35)), ValueRange(UInt64(826), UInt64(875)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(47), Flt64(40)), ValueRange(UInt64(12), UInt64(77)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(45), Flt64(30)), ValueRange(UInt64(734), UInt64(777)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(45), Flt64(35)), ValueRange(UInt64(916), UInt64(969)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(95), Flt64(30)), ValueRange(UInt64(387), UInt64(456)).value!!, UInt64(30), UInt64(90)),
        DemandNode(Point2(Flt64(95), Flt64(35)), ValueRange(UInt64(293), UInt64(360)).value!!, UInt64(20), UInt64(90)),
        DemandNode(Point2(Flt64(53), Flt64(30)), ValueRange(UInt64(450), UInt64(505)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(92), Flt64(30)), ValueRange(UInt64(478), UInt64(551)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(53), Flt64(35)), ValueRange(UInt64(353), UInt64(412)).value!!, UInt64(50), UInt64(90)),
        DemandNode(Point2(Flt64(45), Flt64(65)), ValueRange(UInt64(997), UInt64(1068)).value!!, UInt64(20), UInt64(90)),
        DemandNode(Point2(Flt64(90), Flt64(35)), ValueRange(UInt64(203), UInt64(260)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(88), Flt64(30)), ValueRange(UInt64(574), UInt64(643)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(88), Flt64(35)), ValueRange(UInt64(109), UInt64(170)).value!!, UInt64(20), UInt64(90)),
        DemandNode(Point2(Flt64(87), Flt64(30)), ValueRange(UInt64(668), UInt64(731)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(85), Flt64(25)), ValueRange(UInt64(769), UInt64(820)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(85), Flt64(35)), ValueRange(UInt64(47), UInt64(124)).value!!, UInt64(30), UInt64(90)),
        DemandNode(Point2(Flt64(75), Flt64(55)), ValueRange(UInt64(369), UInt64(420)).value!!, UInt64(20), UInt64(90)),
        DemandNode(Point2(Flt64(72), Flt64(55)), ValueRange(UInt64(265), UInt64(338)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(70), Flt64(58)), ValueRange(UInt64(458), UInt64(523)).value!!, UInt64(20), UInt64(90)),
        DemandNode(Point2(Flt64(68), Flt64(60)), ValueRange(UInt64(555), UInt64(612)).value!!, UInt64(30), UInt64(90)),
        DemandNode(Point2(Flt64(66), Flt64(55)), ValueRange(UInt64(173), UInt64(238)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(65), Flt64(55)), ValueRange(UInt64(85), UInt64(144)).value!!, UInt64(20), UInt64(90)),
        DemandNode(Point2(Flt64(65), Flt64(60)), ValueRange(UInt64(645), UInt64(708)).value!!, UInt64(30), UInt64(90)),
        DemandNode(Point2(Flt64(63), Flt64(58)), ValueRange(UInt64(737), UInt64(802)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(60), Flt64(55)), ValueRange(UInt64(20), UInt64(84)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(60), Flt64(60)), ValueRange(UInt64(836), UInt64(889)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(67), Flt64(85)), ValueRange(UInt64(368), UInt64(441)).value!!, UInt64(20), UInt64(90)),
        DemandNode(Point2(Flt64(65), Flt64(85)), ValueRange(UInt64(475), UInt64(518)).value!!, UInt64(40), UInt64(90)),
        DemandNode(Point2(Flt64(65), Flt64(82)), ValueRange(UInt64(285), UInt64(336)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(62), Flt64(80)), ValueRange(UInt64(196), UInt64(239)).value!!, UInt64(30), UInt64(90)),
        DemandNode(Point2(Flt64(60), Flt64(80)), ValueRange(UInt64(95), UInt64(156)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(60), Flt64(85)), ValueRange(UInt64(561), UInt64(622)).value!!, UInt64(30), UInt64(90)),
        DemandNode(Point2(Flt64(58), Flt64(75)), ValueRange(UInt64(30), UInt64(84)).value!!, UInt64(20), UInt64(90)),
        DemandNode(Point2(Flt64(55), Flt64(80)), ValueRange(UInt64(743), UInt64(820)).value!!, UInt64(10), UInt64(90)),
        DemandNode(Point2(Flt64(55), Flt64(85)), ValueRange(UInt64(647), UInt64(726)).value!!, UInt64(20), UInt64(90)),
        EndNode(Point2(Flt64(40), Flt64(50)), ValueRange(UInt64(0), UInt64(1236)).value!!)
    )

    val vehicles = (0 until 25).map {
        Vehicle(UInt64(200), UInt64(500))
    }

    lateinit var x: BinVariable3
    lateinit var s: URealVariable2

    lateinit var origin: LinearIntermediateSymbols1
    lateinit var destination: LinearIntermediateSymbols1
    lateinit var inFlow: LinearIntermediateSymbols2
    lateinit var outFlow: LinearIntermediateSymbols2
    lateinit var service: LinearIntermediateSymbols1
    lateinit var capacity: LinearIntermediateSymbols1

    val metaModel = LinearMetaModel("demo17")

    private val subProcesses = listOf(
        Demo17::initVariable,
        Demo17::initSymbol,
        Demo17::initObject,
        Demo17::initConstraint,
        Demo17::solve,
        Demo17::analyzeSolution
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
        x = BinVariable3("x", Shape3(nodes.size, nodes.size, vehicles.size))
        for (n1 in nodes) {
            for (n2 in nodes) {
                for (v in vehicles) {
                    val xi = x[n1, n2, v]
                    if (n1 !is EndNode && n2 !is OriginNode && n1 != n2) {
                        xi.name = "${x.name}_(${n1.index},${n2.index},${v.index})"
                        metaModel.add(xi)
                    } else {
                        xi.range.eq(false)
                    }
                }
            }
        }
        s = URealVariable2("s", Shape2(nodes.size, vehicles.size))
        for (n in nodes) {
            for (v in vehicles) {
                val si = s[n, v]
                si.name = "${s.name}_(${n.index}_${v.index})"
            }
        }
        metaModel.add(s)
        return ok
    }

    private suspend fun initSymbol(): Try {
        origin = LinearIntermediateSymbols1("origin", Shape1(vehicles.size)) { i, _ ->
            val v = vehicles[i]
            LinearExpressionSymbol(sum(nodes.filterIsInstance<OriginNode>().flatMap { n1 -> x[n1, _a, v] }), "origin_${v.index}")
        }
        metaModel.add(origin)
        destination = LinearIntermediateSymbols1("destination", Shape1(vehicles.size)) { i, _ ->
            val v = vehicles[i]
            LinearExpressionSymbol(sum(nodes.filterIsInstance<EndNode>().flatMap { n2 -> x[_a, n2, v] }), "destination_${v.index}")
        }
        metaModel.add(destination)
        inFlow = LinearIntermediateSymbols2("in", Shape2(nodes.size, vehicles.size)) { _, vec ->
            val n2 = nodes[vec[0]]
            val v = vehicles[vec[1]]
            if (n2 is OriginNode) {
                LinearExpressionSymbol(LinearPolynomial(), name = "in_(${n2.index},${v.index})")
            } else {
                LinearExpressionSymbol(sum(nodes.filterIsNotInstance<EndNode, Node>().map { n1 -> x[n1, n2, v] }), "in_(${n2.index},${v.index})")
            }
        }
        metaModel.add(inFlow)
        outFlow = LinearIntermediateSymbols2("out", Shape2(nodes.size, vehicles.size)) { _, vec ->
            val n1 = nodes[vec[0]]
            val v = vehicles[vec[1]]
            if (n1 is EndNode) {
                LinearExpressionSymbol(LinearPolynomial(), name = "out_(${n1.index},${v.index})")
            } else {
                LinearExpressionSymbol(sum(nodes.filterIsNotInstance<OriginNode, Node>().map { n2 -> x[n1, n2, v] }), "out_(${n1.index},${v.index})")
            }
        }
        metaModel.add(outFlow)
        service = LinearIntermediateSymbols1("service", Shape1(nodes.size)) { i, _ ->
            val n1 = nodes[i]
            if (n1 is OriginNode || n1 is EndNode) {
                LinearExpressionSymbol(LinearPolynomial(), name = "service_(${n1.index})")
            } else {
                LinearExpressionSymbol(sum(nodes.filterIsNotInstance<OriginNode, Node>().flatMap { n2 -> x[n1, n2, _a] }), "service_(${n1.index})")
            }
        }
        metaModel.add(service)
        capacity = LinearIntermediateSymbols1("capacity", Shape1(vehicles.size)) { i, _ ->
            val v = vehicles[i]
            LinearExpressionSymbol(sum(nodes.flatMap { n1 ->
                nodes.mapNotNull { n2 ->
                    (n2 as? DemandNode)?.demand?.let { it * x[n1, n2, v] }
                }
            }), "capacity_${v.index}")
        }
        metaModel.add(capacity)
        return ok
    }

    private suspend fun initObject(): Try {
        metaModel.minimize(sum(vehicles.map { v -> v.fixedUsedCost * origin[v] }), "used cost")
        metaModel.minimize(sum(nodes.flatMap { n1 ->
            nodes.map { n2 ->
                n1.cost(n2) * sum(x[n1, n2, _a])
            }
        }), "trans cost")
        return ok
    }

    private suspend fun initConstraint(): Try {
        for (v in vehicles) {
            metaModel.addConstraint(
                origin[v] leq 1,
                "origin_${v.index}"
            )
        }
        for (n in nodes.filterIsInstance<DemandNode>()) {
            for (v in vehicles) {
                metaModel.addConstraint(
                    inFlow[n, v] eq outFlow[n, v],
                    "balance_${n.index}_${v.index}",
                )
            }
        }
        for (v in vehicles) {
            metaModel.addConstraint(
                destination[v] leq 1,
                "destination_${v.index}"
            )
        }
        for (n in nodes.filterIsInstance<DemandNode>()) {
            metaModel.addConstraint(
                service[n] eq 1,
                "service_${n.index}"
            )
        }
        val m = nodes.filterIsInstance<EndNode>().maxOf { it.timeWindow.upperBound.value.unwrap() }
        for (n1 in nodes) {
            for (n2 in nodes) {
                for (v in vehicles) {
                    metaModel.addConstraint(
                        s[n1, v] + ((n1 as? DemandNode)?.serviceTime ?: UInt64.zero) + n1.time(n2) - m * (1 - x[n1, n2, v]) leq s[n2, v],
                        "time_window_${n1.index}_${n2.index}_${v.index}"
                    )
                }
            }
        }
        for (n in nodes) {
            for (v in vehicles) {
                metaModel.addConstraint(
                    s[n, v] geq n.timeWindow.lowerBound.value.unwrap(),
                    "time_window_lb_${n.index}_${v.index}"
                )
                metaModel.addConstraint(
                    s[n, v] leq n.timeWindow.upperBound.value.unwrap(),
                    "time_window_ub_${n.index}_${v.index}"
                )
            }
        }
        for (v in vehicles) {
            metaModel.addConstraint(
                capacity[v] leq v.capacity,
                "capacity_${v.index}"
            )
        }
        return ok
    }

    private suspend fun solve(): Try {
        val solver = GurobiLinearSolver(config = SolverConfig(time = 300.seconds))
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
        val route: MutableMap<Vehicle, MutableList<Pair<Node, Node>>> = HashMap()
        val time: MutableMap<Vehicle, MutableMap<Node, UInt64>> = HashMap()
        for (token in metaModel.tokens.tokens) {
            if (token.result!! geq Flt64.one && token.variable belongsTo x) {
                val vector = token.variable.vectorView
                val n1 = nodes[vector[0]]
                val n2 = nodes[vector[1]]
                val v = vehicles[vector[2]]
                route.getOrPut(v) { ArrayList() }.add(n1 to n2)
            }
        }
        for (token in metaModel.tokens.tokens) {
            if (token.result!! geq Flt64.one && token.variable belongsTo s) {
                val vector = token.variable.vectorView
                val n = nodes[vector[0]]
                val v = vehicles[vector[1]]
                time.getOrPut(v) { HashMap() }[n] = token.result!!.round().toUInt64()
            }
        }
        return ok
    }
}
