package fuookami.ospf.kotlin.example.framework_demo.demo2.domain.stowage.model

import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.physics.quantity.*

class Solution(
    val stowage: Map<Position, List<Item>>,
    val predicateLoadWeight: Map<Position, Quantity<Flt64>>,
    val recommendedLoadWeight: Map<Position, Quantity<Flt64>>
)
