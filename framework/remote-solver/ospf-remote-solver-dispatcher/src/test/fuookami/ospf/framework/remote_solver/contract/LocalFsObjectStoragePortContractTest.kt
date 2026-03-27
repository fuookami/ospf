package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.adapter.infrastructure.SystemClockPort
import fuookami.ospf.framework.remote_solver.adapter.localfs.LocalFsObjectStoragePort
import fuookami.ospf.framework.remote_solver.protocol.port.ObjectStoragePort
import java.nio.file.Files
import kotlin.io.path.deleteIfExists

class LocalFsObjectStoragePortContractTest : ObjectStoragePortContractTest() {
    override fun createFixture(): TestFixture<ObjectStoragePort> {
        val tempDir = Files.createTempDirectory("remote-solver-object-contract")
        return TestFixture(
            subject = LocalFsObjectStoragePort(tempDir, SystemClockPort()),
            close = {
                Files.walk(tempDir).use { paths ->
                    paths.sorted(Comparator.reverseOrder()).forEach { it.deleteIfExists() }
                }
            }
        )
    }
}
