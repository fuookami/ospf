#include <ospf/math/algebra/operator/comparison/greater_equal.hpp>

namespace ospf::math::algebra::comparison_operator
{
    template class GreaterEqual<i8>;
    template class GreaterEqual<u8>;
    template class GreaterEqual<i16>;
    template class GreaterEqual<u16>;
    template class GreaterEqual<i32>;
    template class GreaterEqual<u32>;
    template class GreaterEqual<i64>;
    template class GreaterEqual<u64>;
    template class GreaterEqual<i128>;
    template class GreaterEqual<u128>;
    template class GreaterEqual<i256>;
    template class GreaterEqual<u256>;
    template class GreaterEqual<i512>;
    template class GreaterEqual<u512>;
    template class GreaterEqual<i1024>;
    template class GreaterEqual<u1024>;
    template class GreaterEqual<bigint>;

    template class GreaterEqual<f32>;
    template class GreaterEqual<f64>;
    template class GreaterEqual<f128>;
    template class GreaterEqual<f256>;
    template class GreaterEqual<f512>;
    template class GreaterEqual<dec50>;
    template class GreaterEqual<dec100>;
};
