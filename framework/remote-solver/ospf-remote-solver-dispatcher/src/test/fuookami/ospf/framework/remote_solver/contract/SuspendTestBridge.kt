package fuookami.ospf.framework.remote_solver.contract

import kotlinx.coroutines.runBlocking

fun <T> runSuspend(block: suspend () -> T): T = runBlocking { block() }
