package fuookami.ospf.kotlin.example.framework_demo.demo4.infrastructure

import java.util.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.backend.solver.*
import fuookami.ospf.kotlin.core.backend.solver.config.*
import fuookami.ospf.kotlin.core.backend.plugins.cplex.*
import fuookami.ospf.kotlin.core.backend.plugins.gurobi.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*

data object LinearSolverBuilder {
    operator fun invoke(
        solver: String? = null,
        config: SolverConfig = SolverConfig(),
        callBack: Any? = null
    ): LinearSolver {
        return (if (callBack != null) {
            when (callBack) {
                is GurobiLinearSolverCallBack -> {
                    GurobiLinearSolver(
                        config = config,
                        callBack = callBack
                    )
                }

                is CplexSolverCallBack -> {
                    CplexLinearSolver(
                        config = config,
                        callBack = callBack
                    )
                }

                is ScipSolverCallBack -> {
                    ScipLinearSolver(
                        config = config,
                        callBack = callBack
                    )
                }

                else -> {
                    null
                }
            }
        } else if (solver != null) {
            when (solver) {
                "gurobi" -> {
                    GurobiLinearSolver(config = config)
                }

                "cplex" -> {
                    CplexLinearSolver(config = config)
                }

                "scip" -> {
                    ScipLinearSolver(config = config)
                }

                else -> {
                    null
                }
            }
        } else {
            null
        }) ?: if (System.getProperty("os.name").lowercase(Locale.getDefault()).contains("win")) {
            GurobiLinearSolver(
                config = config,
                callBack = GurobiLinearSolverCallBack().afterFailure { grbModel, _, _ ->
                    grbModel.computeIIS()
                    grbModel.write("iis.ilp")
                    ok
                }
            )
        } else {
            GurobiLinearSolver(
                config = config
            )
        }
    }
}
