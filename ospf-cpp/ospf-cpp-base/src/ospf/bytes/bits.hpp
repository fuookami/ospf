#pragma once

#include <ospf/ospf_base_api.hpp>
#include <ospf/basic_definition.hpp>
#include <ospf/literal_constant.hpp>
#include <ospf/type_family.hpp>
#include <bit>

namespace ospf
{
    inline namespace bytes
    {
        namespace bits
        {
            OSPF_BASE_API const u8 reverse_bits_u8(u8 value) noexcept;
            OSPF_BASE_API const u16 reverse_bits_u16(u16 value) noexcept;
            OSPF_BASE_API const u32 reverse_bits_u32(u32 value) noexcept;
            OSPF_BASE_API const u64 reverse_bits_u64(u64 value) noexcept;
        };

        template<typename T>
        inline RetType<T> reverse_bits(ArgCLRefType<T> value) noexcept
        {
            if constexpr (sizeof(T) == 1_uz)
            {
                return std::bit_cast<T>(bits::reverse_bits_u8(std::bit_cast<u8>(value)));
            }
            else if constexpr (sizeof(T) == 2_uz)
            {
                return std::bit_cast<T>(bits::reverse_bits_u16(std::bit_cast<u16>(value)));
            }
            else if constexpr (sizeof(T) == 4_uz)
            {
                return std::bit_cast<T>(bits::reverse_bits_u32(std::bit_cast<u32>(value)));
            }
            else if constexpr (sizeof(T) == 8_uz)
            {
                return std::bit_cast<T>(bits::reverse_bits_u64(std::bit_cast<u64>(value)));
            }
            //else
            //{
            //    static_assert(false, "unsupported bits size.");
            //}
        }
    };
};
