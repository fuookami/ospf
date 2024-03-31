#pragma once

#include <ospf/basic_definition.hpp>
#include <ospf/concepts/with_default.hpp>
#include <ospf/functional/array.hpp>
#include <ospf/literal_constant.hpp>
#include <ospf/random.hpp>
#include <ospf/system_info.hpp>
#include <ospf/type_family.hpp>
#include <span>

namespace ospf
{
    inline namespace bytes
    {
        template<usize len = dynamic_size>
        using Bytes = std::conditional_t<
            len == dynamic_size,
            std::vector<ubyte>,
            std::array<ubyte, len>
        >;

        template<usize len = dynamic_size>
        using BytesView = std::span<const ubyte, len>;

        template<typename T>
            requires std::copyable<T> && std::is_trivially_copyable_v<T>
        inline Bytes<sizeof(T)> to_bytes(ArgCLRefType<T> value, const Endian endian = local_endian) noexcept
        {
            Bytes<sizeof(T)> bytes{ 0_ub };
            const auto ptr = reinterpret_cast<ubyte*>(&value);
            if (endian == local_endian)
            {
                std::copy(ptr, ptr + sizeof(T), bytes.begin());
            }
            else
            {
                std::copy(ptr, ptr + sizeof(T), bytes.rbegin());
            }
            return bytes;
        }

        template<typename T, typename It>
            requires std::copyable<T> && std::is_trivially_copyable_v<T> && std::output_iterator<It, ubyte>
        inline void to_bytes(ArgCLRefType<T> value, It& it, const Endian endian = local_endian) noexcept
        {
            if (endian == local_endian)
            {
                const auto ptr = reinterpret_cast<ubyte*>(&value);
                std::copy(ptr, ptr + sizeof(T), it);
            }
            else
            {
                const auto bytes = sizeof(T);
                const auto ptr = reinterpret_cast<ubyte*>(&value) + bytes - 1_iz;
                for (usize i{ 0_uz }; i != bytes; ++i)
                {
                    *it = *ptr;
                    ++it;
                    --ptr;
                }
            }
        }

        template<typename T>
            requires WithDefault<T> && std::copyable<T> && std::is_trivially_copyable_v<T>
        inline T from_bytes(const Bytes<sizeof(T)>& bytes, const Endian endian = local_endian) noexcept
        {
            T value = DefaultValue<T>::value();
            from_bytes(bytes, value, endian);
            return value;
        }

        template<typename T>
            requires std::copyable<T> && std::is_trivially_copyable_v<T>
        inline void from_bytes(const Bytes<sizeof(T)>& bytes, T& value, const Endian endian = local_endian) noexcept
        {
            const auto ptr = reinterpret_cast<ubyte*>(&value);
            if (endian == local_endian)
            {
                std::copy(bytes.cbegin(), bytes.cend(), ptr);
            }
            else
            {
                std::copy(bytes.crbegin(), bytes.crend(), ptr);
            }
        }

        template<typename T, typename It>
            requires WithDefault<T> && std::copyable<T> && std::is_trivially_copyable_v<T> && std::input_iterator<It>
                && requires (const It& it)
                {
                    { *it } -> DecaySameAs<ubyte>;
                }
        inline T from_bytes(It& it, const Endian endian = local_endian) noexcept
        {
            T value = DefaultValue<T>::value();
            from_bytes(it, value, endian);
            return value;
        }

        template<typename T, typename It>
            requires std::copyable<T> && std::is_trivially_copyable_v<T> && std::input_iterator<It>
                && requires (const It& it)
                {
                    { *it } -> DecaySameAs<ubyte>;
                }
        inline void from_bytes(It& it, T& value, const Endian endian = local_endian) noexcept
        {
            const auto bytes = sizeof(T);
            if (endian == local_endian)
            {
                const auto ptr = reinterpret_cast<ubyte*>(&value);
                for (usize i{ 0_uz }; i != bytes; ++i)
                {
                    *ptr = *it;
                    ++it;
                    ++ptr;
                }
            }
            else
            {
                const auto ptr = reinterpret_cast<ubyte*>(&value) + bytes - 1_iz;
                for (usize i{ 0_uz }; i != bytes; ++i)
                {
                    *ptr = *it;
                    ++it;
                    --ptr;
                }
            }
        }

        template<usize len>
            requires (len != npos)
        inline Bytes<len> random_block(void) noexcept
        {
            auto gen = random_generator<u8>();
            std::uniform_int_distribution<> dis(0, 0xff);
            return make_array<len>([&gen, &dis](const usize _)
                {
                    return static_cast<ubyte>(dis(gen));
                });
        }

        inline Bytes<> random_block(const usize len = address_length) noexcept
        {
            auto gen = random_generator<u8>();
            std::uniform_int_distribution<> dis(0, 0xff);
            Bytes<> ret;
            ret.resize(len, 0_ub);
            for (usize i{ 0_uz }; i != len; ++i)
            {
                ret[i] = static_cast<ubyte>(dis(gen));
            }
            return ret;
        }
    };
};
