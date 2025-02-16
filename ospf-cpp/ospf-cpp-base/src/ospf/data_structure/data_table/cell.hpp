#pragma once

#include <ospf/data_structure/data_table/concepts.hpp>
#include <ospf/data_structure/data_table/header.hpp>
#include <ospf/meta_programming/type_info.hpp>
#include <variant>

namespace ospf
{
    inline namespace data_structure
    {
        namespace data_table
        {
            template<typename C>
            struct CellValueTypeTrait
            {
                inline static const std::optional<std::type_index> type(const C& cell) noexcept
                {
                    return TypeInfo<C>::index();
                }

                template<CharType CharT>
                inline static DataTableHeader<CharT> base_header(std::basic_string<CharT> header) noexcept
                {
                    return DataTableHeader{ std::move(header), TypeInfo<C>::index()};
                }

                template<CharType CharT>
                inline static DataTableHeader<CharT> base_header(const std::basic_string_view<CharT> header) noexcept
                {
                    return DataTableHeader{ std::basic_string<CharT>{ header }, TypeInfo<C>::index() };
                }
            };

            template<typename T>
            struct CellValueTypeTrait<std::optional<T>>
            {
                inline static const std::optional<std::type_index> type(const std::optional<T>& cell) noexcept
                {
                    return TypeInfo<T>::index();
                }

                template<CharType CharT>
                inline static DataTableHeader<CharT> base_header(std::basic_string<CharT> header) noexcept
                {
                    return DataTableHeader{ std::move(header), TypeInfo<T>::index() };
                }

                template<CharType CharT>
                inline static DataTableHeader<CharT> base_header(const std::basic_string_view<CharT> header) noexcept
                {
                    return DataTableHeader{ std::basic_string<CharT>{ header }, TypeInfo<T>::index() };
                }
            };

            template<typename... Ts>
            struct CellValueTypeTrait<std::variant<Ts...>>
            {
                inline static const std::optional<std::type_index> type(const std::variant<Ts...>& cell) noexcept
                {
                    return std::visit([](const auto& v)
                        {
                            return TypeInfo<OriginType<decltype(v)>>::index();
                        }, cell);
                }

                template<CharType CharT>
                inline static DataTableHeader<CharT> base_header(std::basic_string<CharT> header) noexcept
                {
                    return DataTableHeader{ std::move(header), std::set<std::type_index>{ TypeInfo<Ts>::index()... } };
                }

                template<CharType CharT>
                inline static DataTableHeader<CharT> base_header(const std::basic_string_view<CharT> header) noexcept
                {
                    return DataTableHeader{ std::basic_string<CharT>{ header }, std::set<std::type_index>{ TypeInfo<Ts>::index()... } };
                }
            };

            template<typename... Ts>
            struct CellValueTypeTrait<std::optional<std::variant<Ts...>>>
            {
                inline static const std::optional<std::type_index> type(const std::optional<std::variant<Ts...>>& cell) noexcept
                {
                    if (cell.has_value())
                    {
                        return std::visit([](const auto& v)
                            {
                                return TypeInfo<OriginType<decltype(v)>>::index();
                            }, *cell);
                    }
                    else
                    {
                        return std::nullopt;
                    }
                }

                template<CharType CharT>
                inline static DataTableHeader<CharT> base_header(std::basic_string<CharT> header) noexcept
                {
                    return DataTableHeader{ std::move(header), std::set<std::type_index>{ TypeInfo<Ts>::index()... } };
                }

                template<CharType CharT>
                inline static DataTableHeader<CharT> base_header(const std::basic_string_view<CharT> header) noexcept
                {
                    return DataTableHeader{ std::basic_string<CharT>{ header }, std::set<std::type_index>{ TypeInfo<Ts>::index()... } };
                }
            };
        };
    };
};
