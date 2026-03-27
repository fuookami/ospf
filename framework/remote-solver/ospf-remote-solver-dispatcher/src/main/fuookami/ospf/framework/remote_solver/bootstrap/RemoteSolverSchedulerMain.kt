/**
 * RemoteSolverSchedulerMain - 远程求解器调度器主入口
 *
 * RemoteSolverSchedulerMain - Remote solver scheduler main entry point.
 *
 * 远程求解器独立调度器服务的启动入口点。
 * 仅运行调度循环，不提供 HTTP API 服务。
 *
 * Bootstrap entry point for remote solver standalone scheduler service.
 * Runs scheduling loop only, without HTTP API service.
 */
package fuookami.ospf.framework.remote_solver.bootstrap

import kotlinx.coroutines.runBlocking

/**
 * 远程求解器调度器主入口对象
 *
 * Remote solver scheduler main entry object.
 *
 * 启动独立的调度器服务，持续执行任务调度循环。
 * 用于需要单独部署调度器的场景。
 *
 * Starts standalone scheduler service, continuously executing task scheduling loop.
 * Used for scenarios requiring separate scheduler deployment.
 */
object RemoteSolverSchedulerMain {
    /**
     * 主入口方法
     *
     * Main entry method.
     *
     * 启动调度器服务的主方法。
     * 解析配置、创建运行时并运行持续的调度循环。
     *
     * Main method for starting scheduler service.
     * Parses configuration, creates runtime, and runs continuous scheduling loop.
     *
     * @param args 命令行参数
     *             CLI arguments
     */
    @JvmStatic
    fun main(args: Array<String>) {
        val configPath = BootstrapCliSupport.resolveConfigPath(args)
        val properties = BootstrapCliSupport.loadProperties(configPath)
        val runtime = RemoteSolverBootstrapFactory.create(properties = properties)
        val intervalMs = properties["scheduler.loop.interval-ms"]
            ?.trim()
            ?.toLongOrNull()
            ?.coerceAtLeast(10L)
            ?: 200L

        println("remote-solver scheduler started")
        println("config=$configPath")
        println("intervalMs=$intervalMs")

        val running = java.util.concurrent.atomic.AtomicBoolean(true)
        Runtime.getRuntime().addShutdownHook(
            Thread {
                running.set(false)
                println("remote-solver scheduler stopping")
            }
        )

        runBlocking {
            while (running.get()) {
                runtime.service.scheduleOnce()
                Thread.sleep(intervalMs)
            }
        }
    }
}