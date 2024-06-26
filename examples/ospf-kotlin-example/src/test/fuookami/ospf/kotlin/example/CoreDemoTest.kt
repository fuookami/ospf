package fuookami.ospf.kotlin.example

import kotlinx.coroutines.*
import org.junit.jupiter.api.*
import fuookami.ospf.kotlin.example.core_demo.*

class CoreDemoTest {
    @Test
    fun runDemo1() {
        assert(runBlocking { Demo1().ok })
    }

    @Test
    fun runDemo2() {
        assert(runBlocking { Demo2().ok })
    }

    @Test
    fun runDemo3() {
        assert(runBlocking { Demo3().ok })
    }

    @Test
    fun runDemo4() {
        assert(runBlocking { Demo4().ok })
    }

    @Test
    fun runDemo5() {
        assert(runBlocking { Demo5().ok })
    }

    @Test
    fun runDemo6() {
        assert(runBlocking { Demo6().ok })
    }

    @Test
    fun runDemo7() {
        assert(runBlocking { Demo7().ok })
    }
}
