#include <ospf/math/algebra/operator/comparison/greater.hpp>

namespace ospf::math::algebra::comparison_operator
{
    template class Greater<i8>;
    template class Greater<u8>;
    template class Greater<i16>;
    template class Greater<u16>;
    template class Greater<i32>;
    template class Greater<u32>;
    template class Greater<i64>;
    template class Greater<u64>;
    template class Greater<i128>;
    template class Greater<u128>;
    template class Greater<i256>;
    template class Greater<u256>;
    template class Greater<i512>;
    template class Greater<u512>;
    template class Greater<i1024>;
    template class Greater<u1024>;
    template class Greater<bigint>;

    template class Greater<f32>;
    template class Greater<f64>;
    template class Greater<f128>;
    template class Greater<f256>;
    template class Greater<f512>;
    template class Greater<dec50>;
    template class Greater<dec100>;
};
