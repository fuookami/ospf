#pragma once

#include <ospf/concepts/base.hpp>
#include <ospf/string/hasher.hpp>
#include <boost/locale.hpp>
#include <magic_enum.hpp>

namespace ospf
{
    inline namespace concepts
    {
        template<typename T>
        concept EnumType = std::is_enum_v<T>;

        template<EnumType T, CharType CharT = char>
        inline constexpr const std::basic_string_view<CharT> to_string(const T value) noexcept
        {
            if constexpr (SameAs<CharT, char>)
            {
                return magic_enum::enum_name<T>(value);
            }
            else
            {
                static StringHashMap<std::string_view, std::basic_string<CharT>> cache;
                const auto str = magic_enum::enum_name<T>(value);
                auto it = cache.find(str);
                if (it == cache.end())
                {
                    it = cache.insert({ str, boost::locale::conv::to_utf<CharT>(std::string{ str }, std::locale{}) }).first;
                }
                return it->second;
            }
        }

        template<EnumType T, CharType CharT = char>
        inline constexpr std::optional<T> parse_enum(const std::basic_string_view<CharT> str) noexcept
        {
            if constexpr (SameAs<CharT, char>)
            {
                return magic_enum::enum_cast<T>(str);
            }
            else
            {
                static StringHashMap<std::basic_string<CharT>, std::optional<T>> cache;
                auto it = cache.find(str);
                if (it == cache.end())
                {
                    const auto wstr = std::basic_string<CharT>{ str };
                    const auto name = boost::locale::conv::from_utf(wstr, std::locale{});
                    const auto value = magic_enum::enum_cast<T>(name);
                    it = cache.insert({ std::move(wstr), value }).first;
                }
                return it->second;
            }
        }
    };
};

template<ospf::EnumType T>
inline constexpr const bool operator==(const T lhs, const T rhs) noexcept
{
    using ValueType = magic_enum::underlying_type_t<T>;
    return static_cast<ValueType>(lhs) == static_cast<ValueType>(rhs);
}

template<ospf::EnumType T>
inline constexpr const bool operator!=(const T lhs, const T rhs) noexcept
{
    using ValueType = magic_enum::underlying_type_t<T>;
    return static_cast<ValueType>(lhs) != static_cast<ValueType>(rhs);
}

namespace std
{
    template<ospf::EnumType T, ospf::CharType CharT = char>
    inline const basic_string_view<CharT> to_string(const T value) noexcept
    {
        return ospf::to_string<T, CharT>(value);
    }

    template<ospf::EnumType T, ospf::CharType CharT>
    struct formatter<T, CharT>
        : public formatter<basic_string_view<CharT>, CharT>
    {
        template<typename FormatContext>
        inline constexpr decltype(auto) format(const T value, FormatContext& fc) const
        {
            static const auto _formatter = formatter<basic_string_view<CharT>, CharT>{};
            return _formatter.format(ospf::to_string<T, CharT>(value), fc);
        }
    };
};
