#pragma once

#include <ospf/data_structure/data_table.hpp>
#include <ospf/meta_programming/meta_info.hpp>
#include <ospf/meta_programming/name_transfer.hpp>
#include <ospf/serialization/csv/concepts.hpp>
#include <optional>

namespace ospf
{
    inline namespace serialization
    {
        template<CharType CharT = char>
        using CSVTable = DynDataTable<DataTableConfig<StoreType::Row, CharT, on, on>, std::basic_string<CharT>>;

        template<CharType CharT = char>
        using CSVViewTable = DynDataTable<DataTableConfig<StoreType::Row, CharT, on, on>, std::basic_string_view<CharT>>;

        template<usize col, CharType CharT = char>
        using ORMCSVTable = DataTable<col, DataTableConfig<StoreType::Row, CharT, off, on>, std::basic_string<CharT>>;

        template<usize col, CharType CharT = char>
        using ORMCSVViewTable = DataTable<col, DataTableConfig<StoreType::Row, CharT, off, on>, std::basic_string_view<CharT>>;

        namespace csv
        {
            template<WithMetaInfo T, CharType CharT>
            struct ORMCSVTrait
            {
                inline static constexpr const usize col = meta_info::MetaInfo<T>{}.size();
                using HeaderType = std::array<std::basic_string_view<CharT>, col>;
                using RowType = std::array<std::basic_string<CharT>, col>;
                using RowViewType = std::array<std::basic_string_view<CharT>, col>;
                using TableType = ORMCSVTable<col, CharT>;
                using ViewTableType = ORMCSVViewTable<col, CharT>;
            };

            template<WithMetaInfo T, CharType CharT>
            using ORMHeaderType = typename ORMCSVTrait<T, CharT>::HeaderType;

            template<WithMetaInfo T, CharType CharT>
            using ORMRowType = typename ORMCSVTrait<T, CharT>::RowType;

            template<WithMetaInfo T, CharType CharT>
            using ORMRowViewType = typename ORMCSVTrait<T, CharT>::RowViewType;

            template<WithMetaInfo T, CharType CharT>
            using ORMTableType = typename ORMCSVTrait<T, CharT>::TableType;

            template<WithMetaInfo T, CharType CharT>
            using ORMViewTableType = typename ORMCSVTrait<T, CharT>::ViewTableType;

            template<WithMetaInfo T, CharType CharT>
            inline constexpr ORMHeaderType<T, CharT> header(const meta_info::MetaInfo<T>& info, const std::optional<NameTransfer<CharT>>& transfer) noexcept
            {
                ORMHeaderType<T, CharT> header{};
                usize i{ 0_uz };
                info.for_each([&i, &header, &transfer](const auto& field)
                    {
                        header[i] = transfer.has_value() ? (*transfer)(field.key()) : field.key();
                        ++i;
                    });
                return header;
            }
        };

        template<WithMetaInfo T, CharType CharT = char>
        inline csv::ORMTableType<T, CharT> make_csv_table(const std::optional<csv::NameTransfer<CharT>>& transfer = meta_programming::NameTransfer<NamingSystem::SnakeCase, NamingSystem::UpperSnakeCase, CharT>{}) noexcept
        {
            static const meta_info::MetaInfo<T> info{};
            auto header = csv::header(info, transfer);
            return csv::ORMTableType<T, CharT>{ std::span<std::basic_string_view<CharT>, csv::ORMCSVTrait<T, CharT>::col>{ header } };
        }

        template<WithMetaInfo T, CharType CharT = char>
        inline csv::ORMViewTableType<T, CharT> make_csv_view_table(const std::optional<csv::NameTransfer<CharT>>& transfer = meta_programming::NameTransfer<NamingSystem::SnakeCase, NamingSystem::UpperSnakeCase, CharT>{}) noexcept
        {
            static const meta_info::MetaInfo<T> info{};
            auto header = csv::header(info, transfer);
            return csv::ORMViewTableType<T, CharT>{ std::span<std::basic_string_view<CharT>, csv::ORMCSVTrait<T, CharT>::col>{ header } };
        }
    };

    inline namespace data_structure
    {
        namespace data_table
        {
            extern template class DataTable<std::optional<std::string>, dynamic_column, StoreType::Row, char>;
            extern template class DataTable<std::optional<std::string_view>, dynamic_column, StoreType::Row, char>;

            extern template class DataTable<std::string, 1_uz, StoreType::Row, char>;
            extern template class DataTable<std::string, 2_uz, StoreType::Row, char>;
            extern template class DataTable<std::string, 3_uz, StoreType::Row, char>;
            extern template class DataTable<std::string, 4_uz, StoreType::Row, char>;
            extern template class DataTable<std::string, 5_uz, StoreType::Row, char>;
            extern template class DataTable<std::string, 6_uz, StoreType::Row, char>;
            extern template class DataTable<std::string, 7_uz, StoreType::Row, char>;
            extern template class DataTable<std::string, 8_uz, StoreType::Row, char>;
            extern template class DataTable<std::string, 9_uz, StoreType::Row, char>;
            extern template class DataTable<std::string, 10_uz, StoreType::Row, char>;
            extern template class DataTable<std::string, 11_uz, StoreType::Row, char>;
            extern template class DataTable<std::string, 12_uz, StoreType::Row, char>;
            extern template class DataTable<std::string, 13_uz, StoreType::Row, char>;
            extern template class DataTable<std::string, 14_uz, StoreType::Row, char>;
            extern template class DataTable<std::string, 15_uz, StoreType::Row, char>;
            extern template class DataTable<std::string, 16_uz, StoreType::Row, char>;
            extern template class DataTable<std::string, 17_uz, StoreType::Row, char>;
            extern template class DataTable<std::string, 18_uz, StoreType::Row, char>;
            extern template class DataTable<std::string, 19_uz, StoreType::Row, char>;
            extern template class DataTable<std::string, 20_uz, StoreType::Row, char>;

            extern template class DataTable<std::string_view, 1_uz, StoreType::Row, char>;
            extern template class DataTable<std::string_view, 2_uz, StoreType::Row, char>;
            extern template class DataTable<std::string_view, 3_uz, StoreType::Row, char>;
            extern template class DataTable<std::string_view, 4_uz, StoreType::Row, char>;
            extern template class DataTable<std::string_view, 5_uz, StoreType::Row, char>;
            extern template class DataTable<std::string_view, 6_uz, StoreType::Row, char>;
            extern template class DataTable<std::string_view, 7_uz, StoreType::Row, char>;
            extern template class DataTable<std::string_view, 8_uz, StoreType::Row, char>;
            extern template class DataTable<std::string_view, 9_uz, StoreType::Row, char>;
            extern template class DataTable<std::string_view, 10_uz, StoreType::Row, char>;
            extern template class DataTable<std::string_view, 11_uz, StoreType::Row, char>;
            extern template class DataTable<std::string_view, 12_uz, StoreType::Row, char>;
            extern template class DataTable<std::string_view, 13_uz, StoreType::Row, char>;
            extern template class DataTable<std::string_view, 14_uz, StoreType::Row, char>;
            extern template class DataTable<std::string_view, 15_uz, StoreType::Row, char>;
            extern template class DataTable<std::string_view, 16_uz, StoreType::Row, char>;
            extern template class DataTable<std::string_view, 17_uz, StoreType::Row, char>;
            extern template class DataTable<std::string_view, 18_uz, StoreType::Row, char>;
            extern template class DataTable<std::string_view, 19_uz, StoreType::Row, char>;
            extern template class DataTable<std::string_view, 20_uz, StoreType::Row, char>;
        };
    };
};
