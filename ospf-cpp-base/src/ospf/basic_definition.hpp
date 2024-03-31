#pragma once

#include <cstdint>
#include <boost/multiprecision/cpp_int.hpp>
#include <boost/multiprecision/cpp_bin_float.hpp>
#include <boost/multiprecision/cpp_dec_float.hpp>

namespace ospf
{
    using wchar = wchar_t;
    using u8char = char8_t;
    using u16char = char16_t;
    using u32char = char32_t;

    using i8 = std::int8_t;
    using u8 = std::uint8_t;
    using i16 = std::int16_t;
    using u16 = std::uint16_t;
    using i32 = std::int32_t;
    using u32 = std::uint32_t;
    using i64 = std::int64_t;
    using u64 = std::uint64_t;
    using i128 = boost::multiprecision::int128_t;
    using u128 = boost::multiprecision::uint128_t;
    using i256 = boost::multiprecision::int256_t;
    using u256 = boost::multiprecision::uint256_t;
    using i512 = boost::multiprecision::int512_t;
    using u512 = boost::multiprecision::uint512_t;
    using i1024 = boost::multiprecision::int1024_t;
    using u1024 = boost::multiprecision::uint1024_t;

    template<u64 bits = sizeof(void*) * 8>
        requires (bits != 0)
    using intx = boost::multiprecision::number<boost::multiprecision::cpp_int_backend<bits, bits>, boost::multiprecision::expression_template_option::et_off>;
    template<u64 bits = sizeof(void*) * 8>
        requires (bits != 0)
    using uintx = boost::multiprecision::number<boost::multiprecision::cpp_int_backend<bits, bits, boost::multiprecision::cpp_integer_type::unsigned_magnitude>, boost::multiprecision::expression_template_option::et_off>;
    using bigint = boost::multiprecision::number<boost::multiprecision::cpp_int_backend<>, boost::multiprecision::expression_template_option::et_off>;

    using f32 = float;
    using f64 = double;
    using f128 = boost::multiprecision::cpp_bin_float_double_extended;
    using f256 = boost::multiprecision::cpp_bin_float_quad;
    using f512 = boost::multiprecision::cpp_bin_float_oct;

    using dec50 = boost::multiprecision::number<boost::multiprecision::cpp_dec_float<50>, boost::multiprecision::expression_template_option::et_off>;
    using dec100 = boost::multiprecision::number<boost::multiprecision::cpp_dec_float<100>, boost::multiprecision::expression_template_option::et_off>;
    template<u64 digits>
        requires (digits != 0)
    using dec = boost::multiprecision::number<boost::multiprecision::cpp_dec_float<digits, i32, std::allocator<u32>>, boost::multiprecision::expression_template_option::et_off>;

    using ubyte = std::byte;
    using ibyte = std::conditional_t<sizeof(ubyte) == 8, i64, i32>;
    using usize = std::size_t;
    using isize = std::conditional_t<sizeof(usize) == 8, i64, i32>;
    using ptrdiff = std::ptrdiff_t;
    using ptraddr = std::intptr_t;

    static constexpr const auto dynamic_size = static_cast<usize>(-1);
    static constexpr const auto npos = static_cast<usize>(-1);
};
