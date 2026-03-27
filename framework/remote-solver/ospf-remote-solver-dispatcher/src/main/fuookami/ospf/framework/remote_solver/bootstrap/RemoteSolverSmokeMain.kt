@file:OptIn(kotlin.time.ExperimentalTime::class)

/**
 * RemoteSolverSmokeMain - 远程求解器冒烟测试主入口
 *
 * RemoteSolverSmokeMain - Remote solver smoke test main entry point.
 *
 * 远程求解器冒烟测试的启动入口点。
 * 用于快速验证系统基本功能是否正常工作。
 *
 * Bootstrap entry point for remote solver smoke test.
 * Used for quick verification that basic system functionality works correctly.
 */
package fuookami.ospf.framework.remote_solver.bootstrap

import fuookami.ospf.framework.remote_solver.domain.NodeCapabilityProfile
import fuookami.ospf.framework.remote_solver.protocol.domain.NodeId
import fuookami.ospf.framework.remote_solver.protocol.domain.ObjectRef
import fuookami.ospf.framework.remote_solver.protocol.domain.SolvePayload
import fuookami.ospf.framework.remote_solver.protocol.domain.SolverTypeName
import fuookami.ospf.framework.remote_solver.protocol.domain.TaskComplexity
import fuookami.ospf.framework.remote_solver.protocol.domain.TimeSensitivity
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.algebra.number.UInt64
import kotlinx.coroutines.runBlocking
import kotlin.time.DurationUnit
import kotlin.time.toDuration

/**
 * 远程求解器冒烟测试主入口对象
 *
 * Remote solver smoke test main entry object.
 *
 * 执行简单的冒烟测试，验证系统的基本功能。
 * 注册一个节点、提交一个任务并等待完成。
 *
 * Executes simple smoke test, verifying basic system functionality.
 * Registers one node, submits one task, and waits for completion.
 */
object RemoteSolverSmokeMain {
    /**
     * 主入口方法
     *
     * Main entry method.
     *
     * 执行冒烟测试的主方法。
     * 注册测试节点、提交冒烟任务并验证任务完成。
     *
     * Main method for executing smoke test.
     * Registers test node, submits smoke task, and verifies task completion.
     *
     * @param args 命令行参数
     *             CLI arguments
     */
    @JvmStatic
    fun main(args: Array<String>) = runBlocking {
        val configPath = BootstrapCliSupport.resolveConfigPath(args)
        val properties = BootstrapCliSupport.loadProperties(configPath)
        val runtime = RemoteSolverBootstrapFactory.create(properties = properties)

        // 注册冒烟测试节点
        runtime.service.registerNode(
            NodeCapabilityProfile(
                nodeId = NodeId.of("smoke-node-1"),
                solverType = SolverTypeName.of("gurobi"),
                performanceScore = Flt64(1.0),
                pricePerSecond = Flt64(0.1),
                minBillingUnit = 1L.toDuration(DurationUnit.SECONDS),
                supportsInterrupt = true,
                supportsCheckpoint = true,
                supportsWarmStart = true,
                parallelUnits = 1
            )
        )

        // 提交并等待冒烟测试任务完成
        val result = runtime.service.submitAndAwait(
            payload = SolvePayload(
                modelRef = ObjectRef.of(path = "models/smoke-model"),
                extension = mapOf("solverType" to "gurobi")
            ),
            complexity = TaskComplexity.SIMPLE,
            timeSensitivity = TimeSensitivity.NON_REALTIME,
            maxRounds = UInt64(20),
            throwIfNotTerminal = true
        )

        println("smoke task finished")
        println("taskId=${result.taskId}")
        println("status=${result.status}")
        println("consumedCost=${result.consumedCost}")
    }
}
