#pragma once

#include <ospf/string/split.hpp>
#include <typeinfo>
#include <typeindex>

namespace ospf
{
    inline namespace meta_programming
    {
        inline const std::string_view type_name(const std::type_index type) noexcept
        {
            return split(type.name(), " :").back();
        }

        template<typename T>
        struct TypeInfo
        {
        public:
            TypeInfo(void) = delete;

            inline static decltype(auto) info(void) noexcept
            {
                static const auto& ret = typeid(T);
                return ret;
            }

            inline static decltype(auto) index(void) noexcept
            {
                static const auto ret = std::type_index(info());
                return ret;
            }

            inline static const ptraddr code(void) noexcept
            {
                return reinterpret_cast<const ptraddr>(&info());
            }

            inline static const usize hash_code(void) noexcept
            {
                return info().has_code();
            }

            inline static const std::string_view full_name(void) noexcept
            {
                return info().name();
            }

            inline static const std::string_view name(void) noexcept
            {
                static const std::string_view ret = type_name(info());
                return ret;
            }
        };
    };
};
