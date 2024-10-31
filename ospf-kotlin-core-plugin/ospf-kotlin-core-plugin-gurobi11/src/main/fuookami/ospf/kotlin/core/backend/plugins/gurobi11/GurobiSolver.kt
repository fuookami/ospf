package fuookami.ospf.kotlin.core.backend.plugins.gurobi11

import kotlin.time.*
import com.gurobi.gurobi.*
import fuookami.ospf.kotlin.utils.error.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.core.backend.solver.output.*

abstract class GurobiSolver {
    protected lateinit var env: GRBEnv
    protected lateinit var grbModel: GRBModel
    protected lateinit var status: SolverStatus

    protected fun finalize() {
        grbModel.dispose()
        env.dispose()
    }

    protected suspend fun init(server: String, password: String, connectionTime: Duration, name: String): Try {
        return try {
            env = GRBEnv(true)
            env.set(GRB.IntParam.ServerTimeout, connectionTime.toInt(DurationUnit.SECONDS))
            env.set(GRB.DoubleParam.CSQueueTimeout, connectionTime.toDouble(DurationUnit.SECONDS))
            env.set(GRB.StringParam.ComputeServer, server)
            env.set(GRB.StringParam.ServerPassword, password)
            env.start()

            grbModel = GRBModel(env)
            grbModel.set(GRB.StringAttr.ModelName, name)
            ok
        } catch (e: GRBException) {
            Failed(Err(ErrorCode.OREngineEnvironmentLost, e.message))
        } catch (e: Exception) {
            Failed(Err(ErrorCode.OREngineEnvironmentLost))
        }
    }

    protected suspend fun init(name: String): Try {
        return try {
            env = GRBEnv()
            grbModel = GRBModel(env)
            grbModel.set(GRB.StringAttr.ModelName, name)
            ok
        } catch (e: GRBException) {
            Failed(Err(ErrorCode.OREngineEnvironmentLost, e.message))
        } catch (e: Exception) {
            Failed(Err(ErrorCode.OREngineEnvironmentLost))
        }
    }

    protected suspend fun solve(): Try {
        return try {
            grbModel.optimize()

            ok
        } catch (e: GRBException) {
            Failed(Err(ErrorCode.OREngineSolvingException, e.message))
        } catch (e: Exception) {
            Failed(Err(ErrorCode.OREngineTerminated))
        }
    }

    protected suspend fun analyzeStatus(): Try {
        return try {
            status = when (grbModel.get(GRB.IntAttr.Status)) {
                GRB.OPTIMAL -> {
                    SolverStatus.Optimal
                }

                GRB.INFEASIBLE -> {
                    SolverStatus.NoSolution
                }

                GRB.UNBOUNDED -> {
                    SolverStatus.Unbounded
                }

                GRB.INF_OR_UNBD -> {
                    SolverStatus.NoSolution
                }

                else -> {
                    if (grbModel.get(GRB.IntAttr.SolCount) > 0) {
                        SolverStatus.Feasible
                    } else {
                        SolverStatus.NoSolution
                    }
                }
            }

            ok
        } catch (e: GRBException) {
            Failed(Err(ErrorCode.OREngineSolvingException, e.message))
        } catch (e: Exception) {
            Failed(Err(ErrorCode.OREngineSolvingException))
        }
    }
}
