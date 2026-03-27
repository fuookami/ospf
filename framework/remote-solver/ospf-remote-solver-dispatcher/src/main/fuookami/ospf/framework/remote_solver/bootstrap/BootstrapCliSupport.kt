/**
 * BootstrapCliSupport - 启动命令行支持工具
 *
 * BootstrapCliSupport - Bootstrap command-line support utilities.
 *
 * 提供命令行参数解析、配置文件路径解析和属性加载的通用工具方法。
 * 该对象为远程求解器的各种启动入口点提供一致的配置加载机制。
 *
 * Provides common utility methods for CLI argument parsing, config path resolution,
 * and property loading. This object offers a consistent configuration loading mechanism
 * for various bootstrap entry points of the remote solver.
 */
package fuookami.ospf.framework.remote_solver.bootstrap

import java.nio.file.Files
import java.nio.file.Path
import java.util.Properties

/**
 * 启动命令行支持工具对象
 *
 * Bootstrap CLI support utility object.
 *
 * 包含解析命令行参数、定位配置文件和加载配置属性的静态方法。
 * 所有启动入口点（如 ApiMain、SchedulerMain 等）都使用此工具进行初始化。
 *
 * Contains static methods for parsing CLI arguments, locating config files,
 * and loading configuration properties. All bootstrap entry points (such as
 * ApiMain, SchedulerMain, etc.) use this utility for initialization.
 */
object BootstrapCliSupport {
    /**
     * 解析命令行参数为键值对映射
     *
     * Parses CLI arguments into a key-value map.
     *
     * 将命令行参数数组解析为有序的键值对映射。
     * 支持 --key value 格式和 --flag 格式（值为 "true"）。
     *
     * Parses the CLI argument array into an ordered key-value map.
     * Supports both --key value format and --flag format (value is "true").
     *
     * @param args 命令行参数数组
     *             CLI argument array
     * @return 参数键值对映射，保持参数出现的顺序
     *         Parameter key-value map, preserving argument order
     */
    fun parseArgs(args: Array<String>): Map<String, String> {
        val result = linkedMapOf<String, String>()
        var index = 0
        while (index < args.size) {
            val token = args[index]
            if (token.startsWith("--")) {
                val key = token.removePrefix("--")
                if (index + 1 < args.size && !args[index + 1].startsWith("--")) {
                    result[key] = args[index + 1]
                    index += 1
                } else {
                    result[key] = "true"
                }
            }
            index += 1
        }
        return result
    }

    /**
     * 从命令行参数解析配置文件路径
     *
     * Resolves config file path from CLI arguments.
     *
     * 从命令行参数数组中提取 --config 参数指定的配置文件路径。
     * 如果未指定，则使用环境变量 REMOTE_SOLVER_CONFIG 或默认路径。
     *
     * Extracts the config file path specified by --config argument from CLI array.
     * If not specified, uses environment variable REMOTE_SOLVER_CONFIG or default path.
     *
     * @param args 命令行参数数组
     *             CLI argument array
     * @return 配置文件的绝对规范化路径
     *         Absolute normalized path of the config file
     */
    fun resolveConfigPath(args: Array<String>): Path {
        val parsed = parseArgs(args)
        return resolveConfigPath(parsed["config"])
    }

    /**
     * 解析配置文件路径
     *
     * Resolves config file path.
     *
     * 根据提供的路径字符串、环境变量或默认值确定配置文件位置。
     * 优先级：CLI 参数 > 环境变量 > 默认路径。
     *
     * Determines config file location based on provided path string,
     * environment variable, or default value.
     * Priority: CLI argument > Environment variable > Default path.
     *
     * @param cliPath 命令行提供的配置路径，可为 null
     *                Config path provided via CLI, can be null
     * @return 配置文件的绝对规范化路径
     *         Absolute normalized path of the config file
     */
    fun resolveConfigPath(cliPath: String?): Path {
        val path = cliPath?.trim()?.takeIf { it.isNotEmpty() }
            ?: System.getenv("REMOTE_SOLVER_CONFIG")
            ?: "deploy/config/scheduler.properties"
        return Path.of(path).toAbsolutePath().normalize()
    }

    /**
     * 加载配置属性文件
     *
     * Loads configuration properties file.
     *
     * 从指定路径加载 Java Properties 文件并转换为字符串键值对映射。
     * 配置文件必须存在，否则抛出异常。
     *
     * Loads Java Properties file from specified path and converts to
     * string key-value map. Config file must exist, otherwise throws exception.
     *
     * @param path 配置文件路径
     *             Config file path
     * @return 配置属性键值对映射
     *         Configuration properties key-value map
     * @throws IllegalArgumentException 配置文件不存在时抛出
     *                                  Thrown when config file does not exist
     */
    fun loadProperties(path: Path): Map<String, String> {
        require(Files.exists(path)) { "Config file not found: $path" }
        val props = Properties()
        Files.newInputStream(path).use { input ->
            props.load(input)
        }
        return props.entries.associate { (k, v) -> k.toString() to v.toString() }
    }
}