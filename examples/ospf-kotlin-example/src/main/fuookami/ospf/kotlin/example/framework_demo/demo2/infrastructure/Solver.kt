package fuookami.ospf.kotlin.example.framework_demo.demo2.infrastructure

import java.util.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.backend.solver.*
import fuookami.ospf.kotlin.core.backend.solver.config.*
import fuookami.ospf.kotlin.core.backend.plugins.gurobi.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*
import fuookami.ospf.kotlin.framework.solver.*

data object LinearSolverBuilder {
    operator fun invoke(
        solver: String? = null,
        config: SolverConfig = SolverConfig(),
        gurobiConfig: GurobiSolverConfig? = null,
        callBack: Any? = null
    ): AbstractLinearSolver {
        return (if (callBack != null) {
            when (callBack) {
                is GurobiLinearSolverCallBack -> {
                    GurobiLinearSolver(
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
            SerialCombinatorialLinearSolver(
                solvers = listOf(
                    GurobiLinearSolver(
                        config = config.copy(extraConfig = gurobiConfig),
                        callBack = GurobiLinearSolverCallBack().afterFailure { grbModel, _, _ ->
                            grbModel.computeIIS()
                            grbModel.write("iis.ilp")
                            ok
                        }
                    ),
                    ScipLinearSolver(
                        config = config,
                    )
                )
            )

        } else {
            SerialCombinatorialLinearSolver(
                solvers = listOf(
                    GurobiLinearSolver(
                        config = config
                    ),
                    ScipLinearSolver(
                        config = config,
                    )
                )
            )
        }
    }
}
