#pragma once

#include <ospf/ospf_base_api.hpp>
#include <ospf/basic_definition.hpp>
#include <ospf/concepts/base.hpp>
#include <ospf/literal_constant.hpp>
#include <ospf/meta_programming/naming_system.hpp>
#include <locale>
#include <numeric>
#include <span>
#include <set>
#include <string>
#include <string_view>

namespace ospf
{
    inline namespace meta_programming
    {
        namespace name_transfer
        {
            template<NamingSystem system, CharType CharT>
            struct Backend;

            template<CharType CharT>
            struct Backend<NamingSystem::SnakeCase, CharT>
            {
                using StringType = std::basic_string<CharT>;
                using StringViewType = std::basic_string_view<CharT>;

                template<usize len>
                inline StringType operator()(const std::span<const StringViewType, len> words, const std::set<StringViewType>& abbreviations = std::set<StringViewType>{}) const noexcept
                {
                    if (words.empty())
                    {
                        return StringType{};
                    }

                    StringType ret{};
                    const usize size = std::accumulate(words.begin(), words.end(), 0_uz,
                        [](const usize lhs, const StringViewType str)
                        {
                            return lhs + str.size();
                        }) + words.size() - 1_uz;
                    ret.resize(size, CharT{ '_' });

                    for (usize i{ 0_uz }, k{ 0_uz }; i != words.size(); ++i)
                    {
                        const StringViewType word = words[i];
                        assert(!word.empty());
                        for (usize j{ 0_uz }; j != word.size(); ++j, ++k)
                        {
                            ret[k] = std::tolower(word[j], std::locale{});
                        }
                        ++k;
                    }

                    return ret;
                }
            };

            template<CharType CharT>
            struct Backend<NamingSystem::UpperSnakeCase, CharT>
            {
                using StringType = std::basic_string<CharT>;
                using StringViewType = std::basic_string_view<CharT>;

                template<usize len>
                inline StringType operator()(const std::span<const StringViewType, len> words, const std::set<StringViewType>& abbreviations = std::set<StringViewType>{}) const noexcept
                {
                    if (words.empty())
                    {
                        return StringType{};
                    }

                    StringType ret{};
                    const usize size = std::accumulate(words.begin(), words.end(), 0_uz,
                        [](const usize lhs, const StringViewType str)
                        {
                            return lhs + str.size();
                        }) + words.size() - 1_uz;
                        ret.resize(size, CharT{ '_' });
                        for (usize i{ 0_uz }, k{ 0_uz }; i != words.size(); ++i)
                        {
                            const StringViewType word = words[i];
                            assert(!word.empty());
                            for (usize j{ 0_uz }; j != word.size(); ++j, ++k)
                            {
                                ret[k] = std::toupper(word[j], std::locale{});
                            }
                            ++k;
                        }
                        return ret;
                }
            };

            template<CharType CharT>
            struct Backend<NamingSystem::KebabCase, CharT>
            {
                using StringType = std::basic_string<CharT>;
                using StringViewType = std::basic_string_view<CharT>;

                template<usize len>
                inline StringType operator()(const std::span<const StringViewType, len> words, const std::set<StringViewType>& abbreviations = std::set<StringViewType>{}) const noexcept
                {
                    if (words.empty())
                    {
                        return StringType{};
                    }

                    StringType ret{};
                    const usize size = std::accumulate(words.begin(), words.end(), 0_uz,
                        [](const usize lhs, const StringViewType str)
                        {
                            return lhs + str.size();
                        }) + words.size() - 1_uz;
                        ret.resize(size, CharT{ '-' });

                        for (usize i{ 0_uz }, k{ 0_uz }; i != words.size(); ++i)
                        {
                            const StringViewType word = words[i];
                            assert(!word.empty());
                            for (usize j{ 0_uz }; j != word.size(); ++j, ++k)
                            {
                                ret[k] = std::tolower(word[j], std::locale{});
                            }
                            ++k;
                        }

                        return ret;
                }
            };

            template<CharType CharT>
            struct Backend<NamingSystem::CamelCase, CharT>
            {
                using StringType = std::basic_string<CharT>;
                using StringViewType = std::basic_string_view<CharT>;

                template<usize len>
                inline StringType operator()(const std::span<const StringViewType, len> words, const std::set<StringViewType>& abbreviations = std::set<StringViewType>{}) const noexcept
                {
                    if (words.empty())
                    {
                        return StringType{};
                    }

                    StringType ret{};
                    const usize size = std::accumulate(words.begin(), words.end(), 0_uz,
                        [](const usize lhs, const StringViewType str) 
                        { 
                            return lhs + str.size(); 
                        });
                    ret.resize(size, CharT{ ' ' });
                    usize k{ 0_uz };
                    for (usize i{ 0_uz }; i != words.size(); ++i)
                    {
                        const StringViewType word = words[i];
                        assert(!word.empty());
                        if (abbreviations.contains(word) && word.size() <= 3_uz)
                        {
                            for (usize j{ 0_uz }; j != word.size(); ++j, ++k)
                            {
                                ret[k] = std::toupper(word[j], std::locale{});
                            }
                            ++k;
                        }
                        else if (i != 0_uz)
                        {
                            ret[k] = std::toupper(word.front(), std::locale{});
                            ++k;
                            for (usize j{ 1_uz }; j != word.size(); ++j, ++k)
                            {
                                ret[k] = std::tolower(word[j], std::locale{});
                            }
                        }
                        else
                        {
                            for (usize j{ 0_uz }; j != word.size(); ++j, ++k)
                            {
                                ret[k] = std::tolower(word[j], std::locale{});
                            }
                        }
                    }
                    return ret;
                }
            };

            template<CharType CharT>
            struct Backend<NamingSystem::PascalCase, CharT>
            {
                using StringType = std::basic_string<CharT>;
                using StringViewType = std::basic_string_view<CharT>;

                template<usize len>
                inline StringType operator()(const std::span<const StringViewType, len> words, const std::set<StringViewType>& abbreviations = std::set<StringViewType>{}) const noexcept
                {
                    if (words.empty())
                    {
                        return StringType{};
                    }

                    StringType ret{};
                    const usize size = std::accumulate(words.begin(), words.end(), 0_uz,
                        [](const usize lhs, const StringViewType str)
                        {
                            return lhs + str.size();
                        });
                    ret.resize(size, CharT{ ' ' });
                    usize k{ 0_uz };
                    for (usize i{ 0_uz }; i != words.size(); ++i)
                    {
                        const StringViewType word = words[i];
                        assert(!word.empty());
                        if (abbreviations.contains(word) && word.size() <= 3_uz)
                        {
                            for (usize j{ 0_uz }; j != word.size(); ++j, ++k)
                            {
                                ret[k] = std::toupper(word[j], std::locale{});
                            }
                            ++k;
                        }
                        else
                        {
                            ret[k] = std::toupper(word.front(), std::locale{});
                            ++k;
                            for (usize j{ 1_uz }; j != word.size(); ++j, ++k)
                            {
                                ret[k] = std::tolower(word[j], std::locale{});
                            }
                        }
                    }
                    return ret;
                }
            };

            extern template struct Backend<NamingSystem::SnakeCase, char>;
            extern template struct Backend<NamingSystem::SnakeCase, wchar>;

            extern template struct Backend<NamingSystem::UpperSnakeCase, char>;
            extern template struct Backend<NamingSystem::UpperSnakeCase, wchar>;

            extern template struct Backend<NamingSystem::KebabCase, char>;
            extern template struct Backend<NamingSystem::KebabCase, wchar>;

            extern template struct Backend<NamingSystem::CamelCase, char>;
            extern template struct Backend<NamingSystem::CamelCase, wchar>;

            extern template struct Backend<NamingSystem::PascalCase, char>;
            extern template struct Backend<NamingSystem::PascalCase, wchar>;
        };
    };
};
