package fuookami.ospf.kotlin.example.core_demo

import fuookami.ospf.kotlin.utils.concept.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.utils.multi_array.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.intermediate_model.*
// import fuookami.ospf.kotlin.core.backend.plugins.gurobi.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*
import fuookami.ospf.kotlin.core.backend.solver.config.*

class Demo2 {
    class Product : AutoIndexed(Product::class)

    data class Company(
        val cost: Map<Product, Flt64>
    ) : AutoIndexed(Company::class)

    val products: ArrayList<Product> = ArrayList()
    val companies: ArrayList<Company> = ArrayList()

    lateinit var x: BinVariable2
    lateinit var cost: LinearSymbol
    lateinit var assignmentCompany: LinearSymbols1
    lateinit var assignmentProduct: LinearSymbols1

    private val metaModel: LinearMetaModel = LinearMetaModel("demo2")

    companion object {
        val subProcesses = arrayListOf(
            Demo2::initVariable,
            Demo2::initSymbol,
            Demo2::initObject,
            Demo2::initConstraint,
            Demo2::solve,
            Demo2::analyzeSolution
        )
    }

    init {
        products.add(Product())
        products.add(Product())
        products.add(Product())
        products.add(Product())

        companies.add(
            Company(
                mapOf(
                    products[0] to Flt64(920.0),
                    products[1] to Flt64(480.0),
                    products[2] to Flt64(650.0),
                    products[3] to Flt64(340.0)
                )
            )
        )

        companies.add(
            Company(
                mapOf(
                    products[0] to Flt64(870.0),
                    products[1] to Flt64(510.0),
                    products[2] to Flt64(700.0),
                    products[3] to Flt64(350.0)
                )
            )
        )

        companies.add(
            Company(
                mapOf(
                    products[0] to Flt64(880.0),
                    products[1] to Flt64(500.0),
                    products[2] to Flt64(720.0),
                    products[3] to Flt64(400.0)
                )
            )
        )

        companies.add(
            Company(
                mapOf(
                    products[0] to Flt64(930.0),
                    products[1] to Flt64(490.0),
                    products[2] to Flt64(680.0),
                    products[3] to Flt64(410.0)
                )
            )
        )
    }

    suspend operator fun invoke(): Try {
        for (process in subProcesses) {
            when (val result = process(this)) {
                is Failed -> {
                    return Failed(result.error)
                }

                else -> {}
            }
        }
        return Ok(success)
    }

    suspend fun initVariable(): Try {
        x = BinVariable2("x", Shape2(companies.size, products.size))
        for (c in companies) {
            for (p in products) {
                x[c, p]!!.name = "${x.name}_${c.index},${p.index}"
            }
        }
        metaModel.addVars(x)
        return Ok(success)
    }

    suspend fun initSymbol(): Try {
        val costPoly = LinearPolynomial()
        for (c in companies) {
            val products = products.filter { c.cost.contains(it) }
            for (p in products) {
                costPoly += c.cost[p]!! * x[c, p]!!
            }
        }
        cost = LinearSymbol(costPoly, "cost")
        metaModel.addSymbol(cost)

        assignmentCompany = LinearSymbols1("assignmentCompany", Shape1(companies.size))
        for (c in companies) {
            val assignmentCompanyPoly = LinearPolynomial()
            val products = products.filter { c.cost.contains(it) }
            for (p in products) {
                assignmentCompanyPoly += UInt64.one * x[c, p]!!
            }
            assignmentCompany[c.index] = LinearSymbol(assignmentCompanyPoly, "assignment_company_${c.index}")
        }
        metaModel.addSymbols(assignmentCompany)

        assignmentProduct = LinearSymbols1("assignmentProduct", Shape1(products.size))
        for (p in products) {
            val assignmentProductPoly = LinearPolynomial()
            val companies = companies.filter { it.cost.contains(p) }
            for (c in companies) {
                assignmentProductPoly += UInt64.one * x[c, p]!!
            }
            assignmentProduct[p.index] = LinearSymbol(assignmentProductPoly, "assignment_product_${p.index}")
        }
        metaModel.addSymbols(assignmentProduct)
        return Ok(success)
    }

    suspend fun initObject(): Try {
        metaModel.minimize(LinearPolynomial(cost))
        return Ok(success)
    }

    suspend fun initConstraint(): Try {
        for (c in companies) {
            metaModel.addConstraint(assignmentCompany[c]!! leq UInt64.one)
        }
        for (p in products) {
            metaModel.addConstraint(assignmentProduct[p]!! eq UInt64.one)
        }
        return Ok(success)
    }

    suspend fun solve(): Try {
        // val solver = GurobiLinearSolver()
        val solver = SCIPLinearSolver(LinearSolverConfig())
        val model = LinearTriadModel(LinearModel(metaModel))
        when (val ret = solver(model)) {
            is Ok -> {
                metaModel.tokens.setSolution(ret.value.solution)
            }

            is Failed -> {
                return Failed(ret.error)
            }
        }
        return Ok(success)
    }

    suspend fun analyzeSolution(): Try {
        val ret = ArrayList<Pair<Company, Product>>()
        for (token in metaModel.tokens.tokens) {
            if (token.result!! eq Flt64.one
                && token.variable.belongsTo(x)
            ) {
                ret.add(Pair(companies[token.variable.vectorView[0]], products[token.variable.vectorView[1]]))
            }
        }
        return Ok(success)
    }
}
