package fuookami.ospf.framework.remote_solver.contract

import fuookami.ospf.framework.remote_solver.adapter.inmemory.InMemoryMetricsPort

class InMemoryMetricsPortContractTest : MetricsPortContractTest() {
    override fun createFixture(): MetricsPortFixture {
        val port = InMemoryMetricsPort()
        return MetricsPortFixture(
            subject = port,
            counterReader = { name, tags -> port.counter(name, tags) },
            gaugeReader = { name, tags -> port.gaugeValue(name, tags) },
            timingReader = { name, tags -> port.timingValues(name, tags) }
        )
    }
}
