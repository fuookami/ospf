#pragma once

#pragma warning(disable:4455)

#include <ospf/basic_definition.hpp>
#include <cassert>

inline constexpr const ospf::isize operator"" _iz(unsigned long long int value)
{
    assert(value <= 0x8000000000000000);
    return static_cast<ospf::isize>(value);
}

inline constexpr const ospf::usize operator"" _uz(unsigned long long int value)
{
    return static_cast<ospf::usize>(value);
}

inline constexpr const ospf::ubyte operator"" _ub(unsigned long long int value)
{
    assert(value <= 0xff);
    return static_cast<ospf::ubyte>(value);
}

inline constexpr const ospf::u8 operator"" _u8(unsigned long long int value)
{
    assert(value <= 0xff);
    return static_cast<ospf::u8>(value);
}

inline constexpr const ospf::i8 operator"" _i8(unsigned long long int value)
{
    assert(value <= 0x80);
    return static_cast<ospf::i8>(value);
}

inline constexpr const ospf::u16 operator"" _u16(unsigned long long int value)
{
    assert(value <= 0xffff);
    return static_cast<ospf::u16>(value);
}

inline constexpr const ospf::i16 operator"" _i16(unsigned long long int value)
{
    assert(value <= 0x8000);
    return static_cast<ospf::i16>(value);
}

inline constexpr const ospf::u32 operator"" _u32(unsigned long long int value)
{
    assert(value <= 0xffffffff);
    return static_cast<ospf::u32>(value);
}

inline constexpr const ospf::i32 operator"" _i32(unsigned long long int value)
{
    assert(value <= 0x80000000);
    return static_cast<ospf::i32>(value);
}

inline constexpr const ospf::u64 operator"" _u64(unsigned long long int value)
{
    return static_cast<ospf::u64>(value);
}

inline constexpr const ospf::i64 operator"" _i64(unsigned long long int value)
{
    assert(value <= 0x8000000000000000);
    return static_cast<ospf::i64>(value);
}

inline constexpr const ospf::f32 operator"" _f32(long double value)
{
    return static_cast<ospf::f32>(value);
}

inline constexpr const ospf::f64 operator"" _f64(long double value)
{
    return static_cast<ospf::f64>(value);
}
