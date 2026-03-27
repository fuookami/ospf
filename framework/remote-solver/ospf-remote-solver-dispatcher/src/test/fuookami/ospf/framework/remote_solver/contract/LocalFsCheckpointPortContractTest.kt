package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.adapter.localfs.LocalFsCheckpointPort
import fuookami.ospf.framework.remote_solver.protocol.port.CheckpointPort
import java.nio.file.Files
import kotlin.io.path.deleteIfExists

class LocalFsCheckpointPortContractTest : CheckpointPortContractTest() {
    override fun createFixture(): TestFixture<CheckpointPort> {
        val tempDir = Files.createTempDirectory("remote-solver-checkpoint-contract")
        return TestFixture(
            subject = LocalFsCheckpointPort(tempDir),
            close = {
                Files.walk(tempDir).use { paths ->
                    paths.sorted(Comparator.reverseOrder()).forEach { it.deleteIfExists() }
                }
            }
        )
    }
}
