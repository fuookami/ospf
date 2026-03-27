package fuookami.ospf.framework.remote_solver.contract

data class TestFixture<T>(
    val subject: T,
    val close: () -> Unit = {}
)
