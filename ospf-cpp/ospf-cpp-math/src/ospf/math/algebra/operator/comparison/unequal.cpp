#include <ospf/math/algebra/operator/comparison/unequal.hpp>

namespace ospf::math::algebra::comparison_operator
{
    template class Unequal<i8>;
    template class Unequal<u8>;
    template class Unequal<i16>;
    template class Unequal<u16>;
    template class Unequal<i32>;
    template class Unequal<u32>;
    template class Unequal<i64>;
    template class Unequal<u64>;
    template class Unequal<i128>;
    template class Unequal<u128>;
    template class Unequal<i256>;
    template class Unequal<u256>;
    template class Unequal<i512>;
    template class Unequal<u512>;
    template class Unequal<i1024>;
    template class Unequal<u1024>;
    template class Unequal<bigint>;

    template class Unequal<f32>;
    template class Unequal<f64>;
    template class Unequal<f128>;
    template class Unequal<f256>;
    template class Unequal<f512>;
    template class Unequal<dec50>;
    template class Unequal<dec100>;
};
