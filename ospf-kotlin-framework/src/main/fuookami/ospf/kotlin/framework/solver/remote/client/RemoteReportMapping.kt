package fuookami.ospf.kotlin.framework.solver.remote.client

import fuookami.ospf.kotlin.core.solver.report.*
import fuookami.ospf.kotlin.framework.solver.remote.domain.*

internal fun RemoteProblemStatus.toCoreStatus(): ProblemStatus {
    return when (this) {
        RemoteProblemStatus.FEASIBLE -> ProblemStatus.Feasible
        RemoteProblemStatus.INFEASIBLE -> ProblemStatus.Infeasible
        RemoteProblemStatus.UNBOUNDED -> ProblemStatus.Unbounded
        RemoteProblemStatus.INFEASIBLE_OR_UNBOUNDED -> ProblemStatus.InfeasibleOrUnbounded
        RemoteProblemStatus.UNKNOWN -> ProblemStatus.Unknown
    }
}

internal fun RemoteTerminationReason.toCoreReason(): TerminationReason {
    return when (this) {
        RemoteTerminationReason.COMPLETED -> TerminationReason.Completed
        RemoteTerminationReason.TIME_LIMIT -> TerminationReason.TimeLimit
        RemoteTerminationReason.NODE_LIMIT -> TerminationReason.NodeLimit
        RemoteTerminationReason.ITERATION_LIMIT -> TerminationReason.IterationLimit
        RemoteTerminationReason.SOLUTION_LIMIT -> TerminationReason.SolutionLimit
        RemoteTerminationReason.OBJECTIVE_LIMIT -> TerminationReason.ObjectiveLimit
        RemoteTerminationReason.CANCELLED -> TerminationReason.Cancelled
        RemoteTerminationReason.INTERRUPTED -> TerminationReason.Interrupted
        RemoteTerminationReason.NUMERICAL_FAILURE -> TerminationReason.NumericalFailure
        RemoteTerminationReason.BACKEND_FAILURE -> TerminationReason.BackendFailure
    }
}

internal fun RemoteSolutionPresence.toCorePresence(): SolutionPresence {
    return when (this) {
        RemoteSolutionPresence.NONE -> SolutionPresence.None
        RemoteSolutionPresence.INCUMBENT -> SolutionPresence.Incumbent
        RemoteSolutionPresence.OPTIMAL -> SolutionPresence.Optimal
    }
}

internal fun String.asRemoteFingerprint(schemaVersion: String): AuditFingerprint {
    return AuditFingerprint(
        schemaVersion = schemaVersion,
        algorithm = "SHA-256",
        value = this
    )
}
