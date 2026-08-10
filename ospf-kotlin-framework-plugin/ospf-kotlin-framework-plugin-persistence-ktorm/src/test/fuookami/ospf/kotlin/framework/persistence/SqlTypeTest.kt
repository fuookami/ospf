package fuookami.ospf.kotlin.framework.persistence

import java.nio.file.Files
import kotlin.time.Duration
import kotlin.time.Duration.Companion.microseconds
import kotlin.time.Duration.Companion.milliseconds
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertThrows
import org.junit.jupiter.api.Test
import org.ktorm.database.Database
import org.ktorm.dsl.from
import org.ktorm.dsl.insert
import org.ktorm.dsl.select
import org.ktorm.schema.Table

private object DurationTable : Table<Nothing>("t_duration") {
    val durationMs = durationMs("duration_ms")
}

class SqlTypeTest {
    @Test
    fun `durationMs stores whole milliseconds`() {
        val database = createDatabase()
        val duration = 1500.milliseconds + 750.microseconds

        database.insert(DurationTable) {
            set(DurationTable.durationMs, duration)
        }

        database.useConnection { connection ->
            connection.createStatement().use { statement ->
                statement.executeQuery("SELECT duration_ms FROM t_duration").use { resultSet ->
                    assertEquals(true, resultSet.next())
                    assertEquals(1500L, resultSet.getLong("duration_ms"))
                }
            }
        }

        val row = database.from(DurationTable).select().iterator().next()
        assertEquals(1500.milliseconds, row[DurationTable.durationMs])
    }

    @Test
    fun `durationMs rejects values outside the finite SQL range`() {
        val database = createDatabase()

        assertThrows(IllegalArgumentException::class.java) {
            database.insert(DurationTable) {
                set(DurationTable.durationMs, Duration.INFINITE)
            }
        }
        assertThrows(IllegalArgumentException::class.java) {
            database.insert(DurationTable) {
                set(DurationTable.durationMs, -Duration.INFINITE)
            }
        }
    }

    private fun createDatabase(): Database {
        val dbFile = Files.createTempFile("ospf-duration-test", ".db").toFile().apply {
            deleteOnExit()
        }
        return Database.connect("jdbc:sqlite:${dbFile.absolutePath}").also { database ->
            database.useConnection { connection ->
                connection.createStatement().use { statement ->
                    statement.execute(
                        "CREATE TABLE t_duration (duration_ms BIGINT NOT NULL)"
                    )
                }
            }
        }
    }
}
