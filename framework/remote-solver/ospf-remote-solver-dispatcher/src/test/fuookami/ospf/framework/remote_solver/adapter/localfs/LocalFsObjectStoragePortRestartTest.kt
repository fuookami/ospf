package fuookami.ospf.framework.remote_solver.adapter.localfs

import java.nio.file.Files
import java.nio.file.Path
import kotlin.io.path.deleteIfExists
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertNotNull
import kotlinx.coroutines.runBlocking
import fuookami.ospf.framework.remote_solver.adapter.infrastructure.SystemClockPort

/**
 * Local object-storage reconstruction regression tests.
 * 本地对象存储实例重建回归测试。
 */
class LocalFsObjectStoragePortRestartTest {
    /**
     * Verify an object written by one storage instance is readable by another instance.
     * 验证一个存储实例写入的对象可由另一个实例读取。
     */
    @Test
    fun objectSurvivesStoragePortRecreation() = runBlocking {
        val root = Files.createTempDirectory("remote-solver-object-restart")
        try {
            val writer = LocalFsObjectStoragePort(root, SystemClockPort())
            val reference = writer.put(
                path = "tenant-restart/result/task-1/slice-1",
                bytes = "portable-result".encodeToByteArray(),
                metadata = mapOf("schemaVersion" to "2.0")
            )

            val reader = LocalFsObjectStoragePort(root, SystemClockPort())
            val restored = assertNotNull(reader.get(reference))
            assertContentEquals("portable-result".encodeToByteArray(), restored)
        } finally {
            Files.walk(root).use { paths ->
                paths.sorted(Comparator.reverseOrder()).forEach { it.deleteIfExists() }
            }
        }
    }
}
