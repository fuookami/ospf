#pragma once

#include <ospf/ospf_base_api.hpp>
#include <ospf/basic_definition.hpp>
#include <ospf/concepts/base.hpp>
#include <string>
#include <unordered_map>
#include <unordered_set>
#include <functional>

namespace ospf
{
    inline namespace string
    {
        template<CharType CharT>
        struct StringHasher
        {
            using is_transparent = void;

            const usize operator()(const CharT* const str) const noexcept
            {
                static constexpr const auto hasher = std::hash<std::basic_string_view<CharT>>{};
                return hasher(str);
            }

            const usize operator()(const std::basic_string_view<CharT> str) const noexcept
            {
                static constexpr const auto hasher = std::hash<std::basic_string_view<CharT>>{};
                return hasher(str);
            }

            const usize operator()(const std::basic_string<CharT>& str) const noexcept
            {
                static constexpr const auto hasher = std::hash<std::basic_string_view<CharT>>{};
                return hasher(str);
            }
        };

        extern template struct StringHasher<char>;
        extern template struct StringHasher<wchar>;

#ifdef BOOST_MSVC
        template<StringOrViewType K, NotVoidType V>
        using StringHashMap = std::unordered_map<K, V, StringHasher<CharTypeOf<K>>, std::equal_to<>>;

        template<StringOrViewType K, NotVoidType V>
        using StringHashMultiMap = std::unordered_multimap<K, V, StringHasher<CharTypeOf<K>>, std::equal_to<>>;

        template<StringOrViewType K>
        using StringHashSet = std::unordered_set<K, StringHasher<CharTypeOf<K>>, std::equal_to<>>;
#else
        template<StringOrViewType K, NotVoidType V>
        using StringHashMap = std::unordered_map<K, V>;

        template<StringOrViewType K, NotVoidType V>
        using StringHashMultiMap = std::unordered_multimap<K, V>;

        template<StringOrViewType K>
        using StringHashSet = std::unordered_set<K>;
#endif
    };
};
