#pragma once

#include <ospf/concepts/base.hpp>
#include <ospf/data_structure/store_type.hpp>
#include <ospf/meta_programming/variable_type_list.hpp>

namespace ospf
{
    inline namespace data_structure
    {
        namespace data_table
        {
            template<
                typename C,
                usize col,
                StoreType st,
                CharType CharT
            >
            class DataTable;

            template<
                StoreType st,
                CharType CharT,
                typename... Ts
            >
                requires (VariableTypeList<Ts...>::length >= 1_uz)
            class STDataTable;
        };
    };
};
