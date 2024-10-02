package fuookami.ospf.kotlin.example.core_demo

import fuookami.ospf.kotlin.utils.math.*
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

data object Demo10 {
    data class City(
        val name: String
    ) : AutoIndexed(City::class)

    val beginCity = "北京"

    val cities = listOf(
        City("上海"),
        City("合肥"),
        City("广州"),
        City("成都"),
        City("北京")
    )

    val distances = mapOf(
        Pair(cities[0], cities[1]) to Flt64(472.0),
        Pair(cities[0], cities[2]) to Flt64(1520.0),
        Pair(cities[0], cities[3]) to Flt64(2095.0),
        Pair(cities[0], cities[4]) to Flt64(1244.0),

        Pair(cities[1], cities[0]) to Flt64(472.0),
        Pair(cities[1], cities[2]) to Flt64(1257.0),
        Pair(cities[1], cities[3]) to Flt64(1615.0),
        Pair(cities[1], cities[4]) to Flt64(1044.0),

        Pair(cities[2], cities[0]) to Flt64(1529.0),
        Pair(cities[2], cities[1]) to Flt64(1257.0),
        Pair(cities[2], cities[3]) to Flt64(1954.0),
        Pair(cities[2], cities[4]) to Flt64(2174.0),

        Pair(cities[3], cities[0]) to Flt64(2095.0),
        Pair(cities[3], cities[1]) to Flt64(1615.0),
        Pair(cities[3], cities[2]) to Flt64(1954.0),
        Pair(cities[3], cities[4]) to Flt64(1854.0),

        Pair(cities[4], cities[0]) to Flt64(1244.0),
        Pair(cities[4], cities[1]) to Flt64(1044.0),
        Pair(cities[4], cities[2]) to Flt64(2174.0),
        Pair(cities[4], cities[3]) to Flt64(1854.0)
    )

    lateinit var x: BinVariable2
    lateinit var u: IntVariable1

    lateinit var distance: LinearSymbol
    lateinit var depart: LinearSymbols1
    lateinit var reached: LinearSymbols1

    private val metaModel = LinearMetaModel("demo10")

    private val subProcesses = listOf(
        Demo10::initVariable,
        Demo10::initSymbol,
        Demo10::initObject,
        Demo10::initConstraint,
        Demo10::solve,
        Demo10::analyzeSolution
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
        x = BinVariable2("x", Shape2(cities.size, cities.size))
        for (city1 in cities) {
            for (city2 in cities) {
                val xi = x[city1, city2]
                xi.name = "${x.name}_(${city1.name},${city2.name})"
                if (city1 != city2) {
                    metaModel.add(xi)
                } else {
                    xi.range.eq(false)
                }
            }
        }
        u = IntVariable1("u", Shape1(cities.size))
        for (city in cities) {
            val ui = u[city]
            ui.name = "${u.name}_${city.name}"
            if (city.name != beginCity) {
                ui.range.set(ValueRange(Int64(-cities.size.toLong()), Int64(cities.size.toLong())).value!!)
                metaModel.add(ui)
            } else {
                ui.range.eq(Int64.zero)
            }
        }
        return ok
    }

    private suspend fun initSymbol(): Try {
        distance = LinearExpressionSymbol(sum(cities.flatMap { city1 ->
            cities.mapNotNull { city2 ->
                if (city1 == city2) {
                    null
                } else {
                    distances[city1 to city2]?.let { it * x[city1, city2] }
                }
            }
        }), "distance")
        depart = LinearSymbols1("depart", Shape1(cities.size)) { i, _ ->
            val city = cities[i]
            LinearExpressionSymbol(sum(x[city, _a]), "depart_${city.name}")
        }
        reached = LinearSymbols1("reached", Shape1(cities.size)) { i, _ ->
            val city = cities[i]
            LinearExpressionSymbol(sum(x[_a, city]), "reached_${city.name}")
        }
        return ok
    }

    private suspend fun initObject(): Try {
        metaModel.minimize(distance, "distance")
        return ok
    }

    private suspend fun initConstraint(): Try {
        for (city in cities) {
            metaModel.addConstraint(depart[city] eq Flt64.one, "depart_${city.name}")
        }
        for (city in cities) {
            metaModel.addConstraint(reached[city] eq Flt64.one, "reached_${city.name}")
        }
        val notBeginCities = cities.filter { it.name != beginCity }
        for (city1 in notBeginCities) {
            for (city2 in notBeginCities) {
                if (city1 != city2) {
                    metaModel.addConstraint(
                        u[city1] - u[city2] + cities.size * x[city1, city2] leq cities.size - 1,
                        "child_route_(${city1.name},${city2.name})"
                    )
                }
            }
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
        val route: MutableMap<City, City> = hashMapOf()
        for (token in metaModel.tokens.tokens) {
            if (token.result!! eq Flt64.one && token.variable.belongsTo(x)) {
                val vector = token.variable.vectorView
                val city1 = cities[vector[0]]
                val city2 = cities[vector[1]]
                route[city1] = city2
            }
        }
        return ok
    }
}
