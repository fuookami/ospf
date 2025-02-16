#include <ospf/math/algebra/operator/comparison/less.hpp>

namespace ospf::math::algebra::comparison_operator
{
    template class Less<i8>;
    template class Less<u8>;
    template class Less<i16>;
    template class Less<u16>;
    template class Less<i32>;
    template class Less<u32>;
    template class Less<i64>;
    template class Less<u64>;
    template class Less<i128>;
    template class Less<u128>;
    template class Less<i256>;
    template class Less<u256>;
    template class Less<i512>;
    template class Less<u512>;
    template class Less<i1024>;
    template class Less<u1024>;
    template class Less<bigint>;

    template class Less<f32>;
    template class Less<f64>;
    template class Less<f128>;
    template class Less<f256>;
    template class Less<f512>;
    template class Less<dec50>;
    template class Less<dec100>;
};
