package fuookami.ospf.kotlin.framework.bpp3d.domain.item.model

import kotlin.reflect.*
import kotlinx.coroutines.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.math.ordinary.*
import fuookami.ospf.kotlin.utils.math.geometry.*
import fuookami.ospf.kotlin.utils.concept.*
import fuookami.ospf.kotlin.utils.operator.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.framework.bpp3d.infrastructure.*

class PlaneLayer<P : ProjectivePlane>(
    // inherited from Container2<PlaneLayer<P>, P>
    override val shape: Container2Shape<P>,
    override val units: List<ItemPlacement2<P>>
) : Container2<PlaneLayer<P>, P> {
    companion object {
        operator fun <P : ProjectivePlane> invoke(
            space: Container3Shape,
            units: List<ItemPlacement2<P>>,
            plane: P
        ): PlaneLayer<P> {
            return PlaneLayer(
                shape = CommonContainer2Shape(
                    space = space,
                    plane = plane
                ),
                units = units
            )
        }

        operator fun <P : ProjectivePlane, S : Container3<S>> invoke(
            container: Container3<S>,
            units: List<ItemPlacement2<P>>,
            plane: P
        ): PlaneLayer<P> {
            return this(
                space = container.shape,
                units = units,
                plane = plane
            )
        }
    }

    override fun copy() = PlaneLayer(
        shape = shape,
        units = units.map { it.copy() }
    )
}

class BinLayer(
    val iteration: Int,
    val from: KClass<*>,
    // inherited from Container3<BinLayer>
    override val shape: Container3Shape,
    override val units: List<Placement3<*>>,
) : ItemContainer<BinLayer>, ManualIndexed() {
    companion object {
        operator fun invoke(
            iteration: Int,
            layer: PlaneLayer<Side>
        ): BinLayer {
            return BinLayer(
                iteration = iteration,
                from = PlaneLayer::class,
                shape = CommonContainer3Shape(layer),
                units = layer
                    .units
                    .flatMap { it.toPlacement3() }
                    .sortedWithThreeWayComparator { lhs, rhs -> lhs ord rhs }
            )
        }

        operator fun invoke(
            iteration: Int,
            from: KClass<*>,
            bin: BinType,
            units: List<ItemPlacement3>
        ): BinLayer {
            return BinLayer(
                iteration = iteration,
                from = from,
                shape = CommonContainer3Shape(CommonContainer2Shape(bin, Side)),
                units = units.sortedWithThreeWayComparator { lhs, rhs -> lhs ord rhs }
            )
        }
    }

    // inherited from Cuboid<BinLayer>
    override val depth: Flt64 = units.maxOfOrNull { it.maxZ } ?: Flt64.zero

    // inherited from ItemContainer<BinLayer>
    override val bottomOnly: Boolean = true

    override fun copy() = BinLayer(
        iteration = iteration,
        from = from,
        shape = shape,
        units = units.map { it.copy() }
    )

    override fun hashCode() = units.hashCode()

    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (other !is BinLayer) return false

        if (shape != other.shape) return false
        if (units.size != other.units.size) return false
        if (!(items.toTypedArray() contentEquals other.items.toTypedArray())) return false

        return true
    }
}

class PalletLayer(
    val iteration: Int,
    val from: KClass<*>,
    // inherited from Container3<PalletLayer>
    override val shape: Container3Shape,
    override val units: List<Placement3<*>>,
) : ItemContainer<PalletLayer>, ManualIndexed() {
    companion object {
        operator fun invoke(
            iteration: Int,
            layer: PlaneLayer<Bottom>
        ): BinLayer {
            return BinLayer(
                iteration = iteration,
                from = PlaneLayer::class,
                shape = CommonContainer3Shape(layer),
                units = layer
                    .units
                    .flatMap { it.toPlacement3() }
                    .sortedWithThreeWayComparator { lhs, rhs -> lhs ord rhs }
            )
        }

        operator fun invoke(
            iteration: Int,
            from: KClass<*>,
            bin: BinType,
            units: List<ItemPlacement3>
        ): BinLayer {
            return BinLayer(
                iteration = iteration,
                from = from,
                shape = CommonContainer3Shape(CommonContainer2Shape(bin, Bottom)),
                units = units.sortedWithThreeWayComparator { lhs, rhs -> lhs ord rhs }
            )
        }
    }

    // inherited from Cuboid<PalletLayer>
    override val height: Flt64 = units.maxOfOrNull { it.maxY } ?: Flt64.zero

    // inherited from ItemContainer<PalletLayer>
    override val topFlat: Boolean = true

    override fun copy() = PalletLayer(
        iteration = iteration,
        from = from,
        shape = shape,
        units = units.map { it.copy() }
    )

    override fun hashCode() = units.hashCode()

    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (other !is BinLayer) return false

        if (shape != other.shape) return false
        if (units.size != other.units.size) return false
        if (!(units.toTypedArray() contentEquals other.units.toTypedArray())) return false

        return true
    }
}

typealias BinLayerView = CuboidView<BinLayer>
typealias BinLayerPlacement = Placement3<BinLayer>
typealias PalletLayerView = CuboidView<PalletLayer>
typealias PalletLayerPlacement = Placement3<PalletLayer>
