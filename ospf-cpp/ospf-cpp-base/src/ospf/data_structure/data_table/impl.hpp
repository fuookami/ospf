#pragma once

#include <ospf/data_structure/data_table/concepts.hpp>
#include <ospf/data_structure/data_table/header.hpp>
#include <ospf/functional/range_bounds.hpp>
#include <ospf/meta_programming/crtp.hpp>
#include <span>

namespace ospf
{
    inline namespace data_structure
    {
        static constexpr const auto dynamic_column = npos;

        namespace data_table
        {
            using RangeFull = range_bounds::RangeFull;

            template<typename T, typename RV>
            class DataTableRowIterator
            {
            public:
                using TableType = T;
                using RowViewType = OriginType<RV>;

            public:
                inline static const DataTableRowIterator begin(const TableType& table) noexcept
                {
                    return DataTableRowIterator{ 0_uz, table };
                }

                inline static const DataTableRowIterator end(const TableType& table) noexcept
                {
                    return DataTableRowIterator{ table.row(), table };
                }

            private:
                DataTableRowIterator(const usize i, const TableType& table)
                    : _i(i), _table(table) 
                {
                    if (_i > _table->row())
                    {
                        throw OSPFException{ OSPFErrCode::ApplicationError, "out of row range" };
                    }
                }

            public:
                DataTableRowIterator(const DataTableRowIterator& ano) = default;
                DataTableRowIterator(DataTableRowIterator&& ano) noexcept = default;
                DataTableRowIterator& operator=(const DataTableRowIterator& rhs) = default;
                DataTableRowIterator& operator=(DataTableRowIterator&& rhs) noexcept = default;
                ~DataTableRowIterator(void) = default;

            public:
                inline operator const usize(void) const noexcept
                {
                    return _i;
                }

                inline RetType<RowViewType> operator*(void) const
                {
                    return _table->row(_i);
                }

            public:
                inline DataTableRowIterator& operator++(void)
                {
                    next();
                    return *this;
                }

                inline const DataTableRowIterator operator++(int)
                {
                    auto ret = *this;
                    next();
                    return ret;
                }

                inline DataTableRowIterator& operator--(void)
                {
                    last();
                    return *this;
                }

                inline const DataTableRowIterator operator--(int)
                {
                    auto ret = *this;
                    last();
                    return ret;
                }

            public:
                inline const DataTableRowIterator operator+(const ptrdiff diff) const noexcept
                {
                    return DataTableRowIterator{ static_cast<usize>(_i + diff), *_table };
                }

                inline DataTableRowIterator& operator+=(const ptrdiff diff) noexcept
                {
                    const auto temp = static_cast<usize>(_i + diff);
                    if (temp > _table->row())
                    {
                        throw OSPFException{ OSPFErrCode::ApplicationError, "out of row range" };
                    }
                    _i = temp;
                    return *this;
                }

                inline const DataTableRowIterator operator-(const ptrdiff diff) const noexcept
                {
                    return DataTableRowIterator{ static_cast<usize>(_i - diff), *_table };
                }

                inline DataTableRowIterator& operator-=(const ptrdiff diff) noexcept
                {
                    const auto temp = static_cast<usize>(_i - diff);
                    if (temp > _table->row())
                    {
                        throw OSPFException{ OSPFErrCode::ApplicationError, "out of row range" };
                    }
                    _i = temp;
                    return *this;
                }

                inline const ptrdiff operator-(const DataTableRowIterator rhs) const noexcept
                {
                    return _i - rhs._it;
                }

            public:
                inline const bool operator==(const DataTableRowIterator rhs) const noexcept
                {
                    return _i == rhs._i && _table == rhs._table;
                }

                inline const bool operator!=(const DataTableRowIterator rhs) const noexcept
                {
                    return _i != rhs._i || _table != rhs._table;
                }

            public:
                inline const bool operator<(const DataTableRowIterator rhs) const noexcept
                {
                    return _table == rhs._table && _i < rhs._i;
                }

                inline const bool operator<=(const DataTableRowIterator rhs) const noexcept
                {
                    return _table == rhs._table && _i <= rhs._i;
                }

                inline const bool operator>(const DataTableRowIterator rhs) const noexcept
                {
                    return _table == rhs._table && _i > rhs._i;
                }

                inline const bool operator>=(const DataTableRowIterator rhs) const noexcept
                {
                    return _table == rhs._table && _i >= rhs._i;
                }

            public:
                inline std::partial_ordering operator<=>(const DataTableRowIterator rhs) const noexcept
                {
                    if (_table != rhs._table)
                    {
                        return std::partial_ordering::unordered;
                    }
                    else
                    {
                        return _i <=> rhs._i;
                    }
                }

            private:
                inline void next(void)
                {
                    if (_i == _table->row())
                    {
                        throw OSPFException{ OSPFErrCode::ApplicationError, "out of row range" };
                    }
                    ++_i;
                }

                inline void last(void)
                {
                    if (_i == 0_uz)
                    {
                        throw OSPFException{ OSPFErrCode::ApplicationError, "out of row range" };
                    }
                    --_i;
                }

            private:
                usize _i;
                Ref<TableType> _table;
            };

            template<typename T, typename RV>
            class DataTableRowReverseIterator
            {
            public:
                using TableType = T;
                using RowViewType = OriginType<RV>;

            public:
                inline static const DataTableRowReverseIterator begin(const TableType& table) noexcept
                {
                    return DataTableRowReverseIterator{ table.row() - 1_uz, table };
                }

                inline static const DataTableRowReverseIterator end(const TableType& table) noexcept
                {
                    return DataTableRowReverseIterator{ npos, table };
                }

            private:
                DataTableRowReverseIterator(const usize i, const TableType& table)
                    : _i(i), _table(table) 
                {
                    if (_i != npos && _i >= _table->row())
                    {
                        throw OSPFException{ OSPFErrCode::ApplicationError, "out of row range" };
                    }
                }

            public:
                DataTableRowReverseIterator(const DataTableRowReverseIterator& ano) = default;
                DataTableRowReverseIterator(DataTableRowReverseIterator&& ano) noexcept = default;
                DataTableRowReverseIterator& operator=(const DataTableRowReverseIterator& rhs) = default;
                DataTableRowReverseIterator& operator=(DataTableRowReverseIterator&& rhs) noexcept = default;
                ~DataTableRowReverseIterator(void) = default;

            public:
                inline operator const usize(void) const noexcept
                {
                    return _i;
                }

                inline RetType<RowViewType> operator*(void) const
                {
                    return _table->row(_i);
                }

            public:
                inline DataTableRowReverseIterator& operator++(void)
                {
                    next();
                    return *this;
                }

                inline const DataTableRowReverseIterator operator++(int)
                {
                    auto ret = *this;
                    next();
                    return ret;
                }

                inline DataTableRowReverseIterator& operator--(void)
                {
                    last();
                    return *this;
                }

                inline const DataTableRowReverseIterator operator--(int)
                {
                    auto ret = *this;
                    last();
                    return ret;
                }

            public:
                inline const DataTableRowReverseIterator operator+(const ptrdiff diff) const noexcept
                {
                    return DataTableRowReverseIterator{ static_cast<usize>(_i - diff), *_table };
                }

                inline DataTableRowReverseIterator& operator+=(const ptrdiff diff) noexcept
                {
                    const auto temp = static_cast<usize>(_i - diff);
                    if (temp > _table->row())
                    {
                        throw OSPFException{ OSPFErrCode::ApplicationError, "out of row range" };
                    }
                    _i = temp;
                    return *this;
                }

                inline const DataTableRowReverseIterator operator-(const ptrdiff diff) const noexcept
                {
                    return DataTableRowReverseIterator{ static_cast<usize>(_i + diff), *_table };
                }

                inline DataTableRowReverseIterator& operator-=(const ptrdiff diff) noexcept
                {
                    const auto temp = static_cast<usize>(_i + diff);
                    if (temp > _table->row())
                    {
                        throw OSPFException{ OSPFErrCode::ApplicationError, "out of row range" };
                    }
                    _i = temp;
                    return *this;
                }

                inline const ptrdiff operator-(const DataTableRowReverseIterator rhs) const noexcept
                {
                    return rhs._i - _i;
                }

            public:
                inline const bool operator==(const DataTableRowReverseIterator rhs) const noexcept
                {
                    return _i == rhs._i && _table == rhs._table;
                }

                inline const bool operator!=(const DataTableRowReverseIterator rhs) const noexcept
                {
                    return _i != rhs._i || _table != rhs._table;
                }

             public:
                inline const bool operator<(const DataTableRowReverseIterator rhs) const noexcept
                {
                    return _table == rhs._table && _i > rhs._i;
                }

                inline const bool operator<=(const DataTableRowReverseIterator rhs) const noexcept
                {
                    return _table == rhs._table && _i >= rhs._i;
                }

                inline const bool operator>(const DataTableRowReverseIterator rhs) const noexcept
                {
                    return _table == rhs._table && _i < rhs._i;
                }

                inline const bool operator>=(const DataTableRowReverseIterator rhs) const noexcept
                {
                    return _table == rhs._table && _i <= rhs._i;
                }

            public:
                inline std::partial_ordering operator<=>(const DataTableRowReverseIterator rhs) const noexcept
                {
                    if (_table != rhs._table)
                    {
                        return std::partial_ordering::unordered;
                    }
                    else
                    {
                        return rhs._i <=> _i;
                    }
                }

            private:
                inline void next(void)
                {
                    if (_i == npos)
                    {
                        throw OSPFException{ OSPFErrCode::ApplicationError, "out of row range" };
                    }
                    --_i;
                }

                inline void last(void)
                {
                    if (_i == (_table->row() - 1_uz))
                    {
                        throw OSPFException{ OSPFErrCode::ApplicationError, "out of row range" };
                    }
                    ++_i;
                }

            private:
                usize _i;
                Ref<TableType> _table;
            };

            template<typename T, typename CV>
            class DataTableColumnIterator
            {
            public:
                using TableType = T;
                using ColumnViewType = OriginType<CV>;

            public:
                inline static const DataTableColumnIterator begin(const TableType& table) noexcept
                {
                    return DataTableColumnIterator{ 0_uz, table };
                }

                inline static const DataTableColumnIterator end(const TableType& table) noexcept
                {
                    return DataTableColumnIterator{ table.column(), table };
                }

            private:
                DataTableColumnIterator(const usize i, const TableType& table)
                    : _i(i), _table(table) 
                {
                    if (_i > _table->column())
                    {
                        throw OSPFException{ OSPFErrCode::ApplicationError, "out of column range" };
                    }
                }

            public:
                DataTableColumnIterator(const DataTableColumnIterator& ano) = default;
                DataTableColumnIterator(DataTableColumnIterator&& ano) noexcept = default;
                DataTableColumnIterator& operator=(const DataTableColumnIterator& rhs) = default;
                DataTableColumnIterator& operator=(DataTableColumnIterator&& rhs) noexcept = default;
                ~DataTableColumnIterator(void) = default;

            public:
                inline operator const usize(void) const noexcept
                {
                    return _i;
                }

                inline RetType<ColumnViewType> operator*(void) const
                {
                    return _table->column(_i);
                }

            public:
                inline DataTableColumnIterator& operator++(void)
                {
                    next();
                    return *this;
                }

                inline const DataTableColumnIterator operator++(int)
                {
                    auto ret = *this;
                    next();
                    return ret;
                }

                inline DataTableColumnIterator& operator--(void)
                {
                    last();
                    return *this;
                }

                inline const DataTableColumnIterator operator--(int)
                {
                    auto ret = *this;
                    last();
                    return ret;
                }

            public:
                inline const DataTableColumnIterator operator+(const ptrdiff diff) const noexcept
                {
                    return DataTableColumnIterator{ static_cast<usize>(_i + diff), *_table };
                }

                inline DataTableColumnIterator& operator+=(const ptrdiff diff) noexcept
                {
                    const auto temp = static_cast<usize>(_i + diff);
                    if (temp > _table->column())
                    {
                        throw OSPFException{ OSPFErrCode::ApplicationError, "out of column range" };
                    }
                    _i = temp;
                    return *this;
                }

                inline const DataTableColumnIterator operator-(const ptrdiff diff) const noexcept
                {
                    return DataTableColumnIterator{ static_cast<usize>(_i - diff), *_table };
                }

                inline DataTableColumnIterator& operator-=(const ptrdiff diff) noexcept
                {
                    const auto temp = static_cast<usize>(_i - diff);
                    if (temp > _table->column())
                    {
                        throw OSPFException{ OSPFErrCode::ApplicationError, "out of column range" };
                    }
                    _i = temp;
                    return *this;
                }

                inline const ptrdiff operator-(const DataTableColumnIterator rhs) const noexcept
                {
                    return _i - rhs._i;
                }

            public:
                inline const bool operator==(const DataTableColumnIterator rhs) const noexcept
                {
                    return _i == rhs._i && _table == rhs._table;
                }

                inline const bool operator!=(const DataTableColumnIterator rhs) const noexcept
                {
                    return _i != rhs._i || _table != rhs._table;
                }

            public:
                inline const bool operator<(const DataTableColumnIterator rhs) const noexcept
                {
                    return _table == rhs._table && _i < rhs._i;
                }

                inline const bool operator<=(const DataTableColumnIterator rhs) const noexcept
                {
                    return _table == rhs._table && _i <= rhs._i;
                }

                inline const bool operator>(const DataTableColumnIterator rhs) const noexcept
                {
                    return _table == rhs._table && _i > rhs._i;
                }

                inline const bool operator>=(const DataTableColumnIterator rhs) const noexcept
                {
                    return _table == rhs._table && _i >= rhs._i;
                }

            public:
                inline std::partial_ordering operator<=>(const DataTableColumnIterator rhs) const noexcept
                {
                    if (_table != rhs._table)
                    {
                        return std::partial_ordering::unordered;
                    }
                    else
                    {
                        return _i <=> rhs._i;
                    }
                }

            private:
                inline void next(void)
                {
                    if (_i == _table->column())
                    {
                        throw OSPFException{ OSPFErrCode::ApplicationError, "out of column range" };
                    }
                    ++_i;
                }

                inline void last(void)
                {
                    if (_i == 0_uz)
                    {
                        throw OSPFException{ OSPFErrCode::ApplicationError, "out of column range" };
                    }
                    --_i;
                }

            private:
                usize _i;
                Ref<TableType> _table;
            };

            template<typename T, typename CV>
            class DataTableColumnReverseIterator
            {
            public:
                using TableType = T;
                using ColumnViewType = OriginType<CV>;

            public:
                inline static const DataTableColumnReverseIterator begin(const TableType& table) noexcept
                {
                    return DataTableColumnReverseIterator{ table.column() - 1_uz, table};
                }

                inline static const DataTableColumnReverseIterator end(const TableType& table) noexcept
                {
                    return DataTableColumnReverseIterator{ npos, table };
                }

            private:
                DataTableColumnReverseIterator(const usize i, const TableType& table)
                    : _i(i), _table(table) 
                {
                    if (i != npos && i >= _table->column())
                    {
                        throw OSPFException{ OSPFErrCode::ApplicationError, "out of column range" };
                    }
                }

            public:
                DataTableColumnReverseIterator(const DataTableColumnReverseIterator& ano) = default;
                DataTableColumnReverseIterator(DataTableColumnReverseIterator&& ano) noexcept = default;
                DataTableColumnReverseIterator& operator=(const DataTableColumnReverseIterator& rhs) = default;
                DataTableColumnReverseIterator& operator=(DataTableColumnReverseIterator&& rhs) noexcept = default;
                ~DataTableColumnReverseIterator(void) = default;

            public:
                inline operator const usize(void) const noexcept
                {
                    return _i;
                }

                inline RetType<ColumnViewType> operator*(void) const
                {
                    return _table->column(_i);
                }

            public:
                inline DataTableColumnReverseIterator& operator++(void)
                {
                    next();
                    return *this;
                }

                inline const DataTableColumnReverseIterator operator++(int)
                {
                    auto ret = *this;
                    next();
                    return ret;
                }

                inline DataTableColumnReverseIterator& operator--(void)
                {
                    last();
                    return *this;
                }

                inline const DataTableColumnReverseIterator operator--(int)
                {
                    auto ret = *this;
                    last();
                    return ret;
                }

            public:
                inline const DataTableColumnReverseIterator operator+(const ptrdiff diff) const noexcept
                {
                    return DataTableColumnReverseIterator{ static_cast<usize>(_i - diff), *_table };
                }

                inline DataTableColumnReverseIterator& operator+=(const ptrdiff diff) noexcept
                {
                    const auto temp = static_cast<usize>(_i - diff);
                    if (temp > _table->column())
                    {
                        throw OSPFException{ OSPFErrCode::ApplicationError, "out of column range" };
                    }
                    _i = temp;
                    return *this;
                }

                inline const DataTableColumnReverseIterator operator-(const ptrdiff diff) const noexcept
                {
                    return DataTableColumnReverseIterator{ static_cast<usize>(_i + diff), *_table };
                }

                inline DataTableColumnReverseIterator& operator-=(const ptrdiff diff) noexcept
                {
                    const auto temp = static_cast<usize>(_i + diff);
                    if (temp > _table->column())
                    {
                        throw OSPFException{ OSPFErrCode::ApplicationError, "out of column range" };
                    }
                    _i = temp;
                    return *this;
                }

                inline const ptrdiff operator-(const DataTableColumnReverseIterator rhs) const noexcept
                {
                    return rhs._i - _i;
                }

            public:
                inline const bool operator==(const DataTableColumnReverseIterator rhs) const noexcept
                {
                    return _i == rhs._i && _table == rhs._table;
                }

                inline const bool operator!=(const DataTableColumnReverseIterator rhs) const noexcept
                {
                    return _i != rhs._i || _table != rhs._table;
                }

            public:
                inline const bool operator<(const DataTableColumnReverseIterator rhs) const noexcept
                {
                    return _table == rhs._table && _i > rhs._i;
                }

                inline const bool operator<=(const DataTableColumnReverseIterator rhs) const noexcept
                {
                    return _table == rhs._table && _i >= rhs._i;
                }

                inline const bool operator>(const DataTableColumnReverseIterator rhs) const noexcept
                {
                    return _table == rhs._table && _i < rhs._i;
                }

                inline const bool operator>=(const DataTableColumnReverseIterator rhs) const noexcept
                {
                    return _table == rhs._table && _i <= rhs._i;
                }

            public:
                inline std::partial_ordering operator<=>(const DataTableColumnReverseIterator rhs) const noexcept
                {
                    if (_table != rhs._table)
                    {
                        return std::partial_ordering::unordered;
                    }
                    else
                    {
                        return rhs._i <=> _i;
                    }
                }

            private:
                inline void next(void)
                {
                    if (_i == npos)
                    {
                        throw OSPFException{ OSPFErrCode::ApplicationError, "out of column range" };
                    }
                    --_i;
                }

                inline void last(void)
                {
                    if (_i == (_table->column() - 1_uz))
                    {
                        throw OSPFException{ OSPFErrCode::ApplicationError, "out of column range" };
                    }
                    ++_i;
                }

            private:
                usize _i;
                Ref<TableType> _table;
            };

            template<typename F, typename CellType, typename CharT>
            concept RowConstructorType = CharType<CharT> 
                && requires (const F& fun, const usize col, const DataTableHeader<CharT>& header)
                {
                    { fun(col, header) } -> DecaySameAs<CellType>;
                };

            template<CharType CharT, typename C, typename T, typename RV>
            class DataTableRows
            {
            public:
                using TableType = T;
                using CellType = OriginType<C>;
                using RowViewType = OriginType<RV>;
                using RowConstructor = std::function<RetType<CellType>(const usize, const DataTableHeader<CharT>&)>;
                using IterType = DataTableRowIterator<TableType, RowViewType>;
                using ReverseIterType = DataTableRowReverseIterator<TableType, RowViewType>;

            public:
                DataTableRows(const TableType& table)
                    : _table(table) {}
                DataTableRows(const DataTableRows& ano) = default;
                DataTableRows(DataTableRows&& ano) noexcept = default;
                DataTableRows& operator=(const DataTableRows& rhs) = default;
                DataTableRows& operator=(DataTableRows&& rhs) noexcept = default;
                ~DataTableRows(void) noexcept = default;

            public:
                inline RetType<RowViewType> operator[](const usize i) const
                {
                    return _table->row(i);
                }

            public:
                inline RetType<IterType> begin(void) const noexcept
                {
                    return IterType::begin(*_table);
                }

                inline RetType<IterType> cbegin(void) const noexcept
                {
                    return IterType::end(*_table);
                }

                inline RetType<IterType> end(void) const noexcept
                {
                    return IterType::end(*_table);
                }

                inline RetType<IterType> cend(void) const noexcept
                {
                    return IterType::end(*_table);
                }

            public:
                inline RetType<ReverseIterType> rbegin(void) const noexcept
                {
                    return ReverseIterType::begin(*_table);
                }

                inline RetType<ReverseIterType> crbegin(void) const noexcept
                {
                    return ReverseIterType::begin(*_table);
                }

                inline RetType<ReverseIterType> rend(void) const noexcept
                {
                    return ReverseIterType::end(*_table);
                }

                inline RetType<ReverseIterType> crend(void) const noexcept
                {
                    return ReverseIterType::end(*_table);
                }

            public:
                template<typename = void>
                    requires WithDefault<CellType>
                inline const usize insert(const usize pos)
                {
                    return _table->insert_row(pos);
                }

                inline const usize insert(const usize pos, ArgCLRefType<CellType> value)
                {
                    return _table->insert_row(pos, value);
                }

                template<typename U>
                inline const usize insert(const usize pos, U&& value)
                {
                    return _table->insert_row(pos, std::forward<U>(value));
                }

                template<typename F>
                    requires requires (const F& fun, const usize col) 
                    { 
                        { fun(col) } -> DecaySameAs<CellType>; 
                    }
                inline const usize insert(const usize pos, const F& constructor)
                {
                    return _table->insert_row(pos, constructor);
                }

                template<typename F>
                    requires RowConstructorType<F, CellType, CharT>
                inline const usize insert(const usize pos, const F& constructor)
                {
                    return _table->insert_row(pos, constructor);
                }

                template<typename = void>
                    requires WithDefault<CellType>
                inline RetType<IterType> insert(ArgCLRefType<IterType> pos)
                {
                    return _table->insert_row(pos);
                }

                inline RetType<IterType> insert(ArgCLRefType<IterType> pos, ArgCLRefType<CellType> value)
                {
                    return _table->insert_row(pos, value);
                }

                template<typename U>
                inline RetType<IterType> insert(ArgCLRefType<IterType> pos, U&& value)
                {
                    return _table->insert_row(pos, std::forward<U>(value));
                }

                template<typename F>
                    requires requires (const F& fun, const usize col)
                    { 
                        { fun(col) } -> DecaySameAs<CellType>; 
                    }
                inline RetType<IterType> insert(ArgCLRefType<IterType> pos, const F& constructor)
                {
                    return _table->insert_row(pos, constructor);
                }

                template<typename F>
                    requires RowConstructorType<F, CellType, CharT>
                inline RetType<IterType> insert(ArgCLRefType<IterType> pos, const F& constructor)
                {
                    return _table->insert_row(pos, constructor);
                }

                inline RetType<IterType> erase(const usize pos) noexcept
                {
                    return _table->erase_row(pos);
                }

                inline RetType<IterType> erase(ArgCLRefType<IterType> pos) noexcept
                {
                    return _table->erase_row(pos);
                }

            private:
                Ref<TableType> _table;
            };

            template<typename F, typename CellType>
            concept ColumnConstructorType = requires (const F& fun, const usize row)
            {
                { fun(row) } -> DecaySameAs<CellType>;
            };

            template<CharType CharT, typename C, typename T, typename CV>
            class DataTableColumns
            {
            public:
                using TableType = T;
                using CellType = OriginType<C>;
                using ColumnViewType = OriginType<CV>;
                using ColumnConstructor = std::function<RetType<CellType>(const usize)>;
                using IterType = DataTableColumnIterator<TableType, ColumnViewType>;
                using ReverseIterType = DataTableColumnReverseIterator<TableType, ColumnViewType>;

            public:
                DataTableColumns(const TableType& table)
                    : _table(table) {}
                DataTableColumns(const DataTableColumns& ano) = default;
                DataTableColumns(DataTableColumns&& ano) noexcept = default;
                DataTableColumns& operator=(const DataTableColumns& rhs) = default;
                DataTableColumns& operator=(DataTableColumns&& rhs) noexcept = default;
                ~DataTableColumns(void) noexcept = default;

            public:
                inline RetType<ColumnViewType> operator[](const usize i) const
                {
                    return _table->column(i);
                }

            public:
                inline RetType<IterType> begin(void) const noexcept
                {
                    return IterType::begin(*_table);
                }

                inline RetType<IterType> cbegin(void) const noexcept
                {
                    return IterType::end(*_table);
                }

                inline RetType<IterType> end(void) const noexcept
                {
                    return IterType::end(*_table);
                }

                inline RetType<IterType> cend(void) const noexcept
                {
                    return IterType::end(*_table);
                }

            public:
                inline RetType<ReverseIterType> rbegin(void) const noexcept
                {
                    return ReverseIterType::begin(*_table);
                }

                inline RetType<ReverseIterType> crbegin(void) const noexcept
                {
                    return ReverseIterType::begin(*_table);
                }

                inline RetType<ReverseIterType> rend(void) const noexcept
                {
                    return ReverseIterType::end(*_table);
                }

                inline RetType<ReverseIterType> crend(void) const noexcept
                {
                    return ReverseIterType::end(*_table);
                }

            public:
                template<typename = void>
                    requires WithDefault<CellType> 
                        && requires (TableType& table) 
                        { 
                            table.insert_column(std::declval<usize>(), std::declval<DataTableHeader<CharT>>());
                        }
                inline const usize insert(const usize pos, ArgRRefType<DataTableHeader<CharT>> header)
                {
                    return _table->insert_column(pos, move<DataTableHeader<CharT>>(header));
                }

                template<typename = void>
                    requires requires (TableType& table) 
                        { 
                            table.insert_column(std::declval<usize>(), std::declval<DataTableHeader<CharT>>(), std::declval<CellType>()); 
                        }
                inline const usize insert(const usize pos, ArgRRefType<DataTableHeader<CharT>> header, ArgCLRefType<CellType> value)
                {
                    return _table->insert_column(pos, move<DataTableHeader<CharT>>(header), value);
                }

                template<typename U>
                    requires requires (TableType& table) 
                        { 
                            table.insert_column(std::declval<usize>(), std::declval<DataTableHeader<CharT>>(), std::declval<U>()); 
                        }
                inline const usize insert(const usize pos, ArgRRefType<DataTableHeader<CharT>> header, U&& value)
                {
                    return _table->insert_column(pos, move<DataTableHeader<CharT>>(header), std::forward<U>(value));
                }

                template<typename F>
                    requires ColumnConstructorType<F, CellType>
                        && requires (TableType& table) 
                        { 
                            table.insert_column(std::declval<usize>(), std::declval<DataTableHeader<CharT>>(), std::declval<F>());
                        }
                inline const usize insert(const usize pos, ArgRRefType<DataTableHeader<CharT>> header, const F& constructor)
                {
                    return _table->insert_column(pos, move<DataTableHeader<CharT>>(header), constructor);
                }

                template<typename = void>
                    requires WithDefault<CellType> 
                        && requires (TableType& table) 
                        { 
                            table.insert_column(std::declval<IterType>(), std::declval<DataTableHeader<CharT>>());
                        }
                inline RetType<IterType> insert(ArgCLRefType<IterType> pos, ArgRRefType<DataTableHeader<CharT>> header)
                {
                    return _table->insert_column(pos, move<DataTableHeader<CharT>>(header));
                }

                template<typename = void>
                    requires requires (TableType& table) 
                        { 
                            table.insert_column(std::declval<IterType>(), std::declval<DataTableHeader<CharT>>(), std::declval<CellType>()); 
                        }
                inline RetType<IterType> insert(ArgCLRefType<IterType> pos, ArgRRefType<DataTableHeader<CharT>> header, ArgCLRefType<CellType> value)
                {
                    return _table->insert_column(pos, move<DataTableHeader<CharT>>(header), value);
                }

                template<typename U>
                    requires requires (TableType& table) 
                        { 
                            table.insert_column(std::declval<IterType>(), std::declval<DataTableHeader<CharT>>(), std::declval<U>()); 
                        }
                inline RetType<IterType> insert(ArgCLRefType<IterType> pos, ArgRRefType<DataTableHeader<CharT>> header, U&& value)
                {
                    return _table->insert_column(pos, move<DataTableHeader<CharT>>(header), std::forward<U>(value));
                }

                template<typename F>
                    requires ColumnConstructorType<F, CellType>
                        && requires (TableType& table) 
                        { 
                            table.insert_column(std::declval<IterType>(), std::declval<DataTableHeader<CharT>>(), std::declval<F>());
                        }
                inline RetType<IterType> insert(ArgCLRefType<IterType> pos, ArgRRefType<DataTableHeader<CharT>> header, const F& constructor)
                {
                    return _table->insert_column(pos, move<DataTableHeader<CharT>>(header), constructor);
                }

                template<typename = void>
                    requires requires (TableType& table) 
                        { 
                            table.erase_column(std::declval<usize>()); 
                        }
                inline const usize erase(const usize pos) noexcept
                {
                    return _table->erase_column(pos);
                }

                template<typename = void>
                    requires requires (TableType& table) 
                        { 
                            table.erase_column(std::declval<IterType>()); 
                        }
                inline RetType<IterType> erase(ArgCLRefType<IterType> pos) noexcept
                {
                    return _table->erase_column(pos);
                }

            private:
                Ref<TableType> _table;
            };

            template<
                StoreType st,
                typename C, 
                typename T
            >
            class DataTableCellWrapper
            {
            public:
                using CellType = OriginType<C>;
                using TableType = T;

            public:
                DataTableCellWrapper(const TableType& table, const usize row, const usize col)
                    : _row(row), _col(col), _table(table) {}
            public:
                DataTableCellWrapper(const DataTableCellWrapper& ano) = delete;
                DataTableCellWrapper(DataTableCellWrapper&& ano) noexcept = default;
                DataTableCellWrapper& operator=(const DataTableCellWrapper& rhs) = delete;
                DataTableCellWrapper& operator=(DataTableCellWrapper&& rhs) noexcept = default;
                ~DataTableCellWrapper(void) = default;

            public:
                template<typename U>
                DataTableCellWrapper& operator=(U&& value)
                {
#ifdef _DEBUG
                    const auto& header = _table->header()[_col];
                    if (header.empty())
                    {
                        throw OSPFException{ OSPFErrCode::ApplicationError, std::format("header of column {} is uninitialized", _col) };
                    }
                    else if (!header.matched<OriginType<U>>())
                    {
                        throw OSPFException{ OSPFErrCode::ApplicationError, std::format("type {} is not matched header of column {}: {}", TypeInfo<OriginType<U>>::name(),  _col, header) };
                    }
#endif
                    auto& cell = static_cast<CellType&>(*this);
                    cell = CellType{ std::forward<U>(value) };
                    return *this;
                }

            public:
                inline constexpr operator LRefType<CellType>(void) const
                {
                    if constexpr (st == StoreType::Row)
                    {
                        return const_cast<CellType&>(_table->body()[_row][_col]);
                    }
                    else
                    {
                        return const_cast<CellType&>(_table->body()[_col][_row]);
                    }
                }

            private:
                usize _row;
                usize _col;
                Ref<TableType> _table;
            };

            template<
                usize col,
                StoreType st,
                CharType CharT,
                typename C,
                typename H,
                typename RV,
                typename CV,
                typename T,
                typename Self
            >
            class DataTableImpl
            {
                OSPF_CRTP_IMPL

            public:
                static constexpr const StoreType store_type = st;
                using CellType = OriginType<C>;
                using HeaderType = OriginType<H>;
                using HeaderViewType = std::span<const DataTableHeader<CharT>, col>;
                using RowViewType = OriginType<RV>;
                using ColumnViewType = OriginType<CV>;
                using TableType = OriginType<T>;
                using CellWrapperType = DataTableCellWrapper<st, CellType, TableType>;
                using RowIterType = DataTableRowIterator<Self, RowViewType>;
                using RowReverseIterType = DataTableRowReverseIterator<Self, RowViewType>;
                using ColumnIterType = DataTableColumnIterator<Self, ColumnViewType>;
                using ColumnReverseIterType = DataTableColumnReverseIterator<Self, ColumnViewType>;
                using RowsViewType = DataTableRows<CharT, CellType, Self, RowViewType>;
                using ColumnsViewType = DataTableColumns<CharT, CellType, Self, ColumnViewType>;
                using RowConstructor = typename RowsViewType::RowConstructor;
                using ColumnConstructor = typename ColumnsViewType::ColumnConstructor;

                using StringType = std::basic_string<CharT>;
                using StringViewType = std::basic_string_view<CharT>;

            protected:
                DataTableImpl(void) = default;
            public:
                DataTableImpl(const DataTableImpl& ano) = default;
                DataTableImpl(DataTableImpl&& ano) noexcept = default;
                DataTableImpl& operator=(const DataTableImpl& rhs) = default;
                DataTableImpl& operator=(DataTableImpl&& rhs) noexcept = default;
                ~DataTableImpl(void) = default;

            public:
                inline const bool empty(void) const noexcept
                {
                    if constexpr (st == StoreType::Row)
                    {
                        return body().empty();
                    }
                    else
                    {
                        return body().empty() ? true : body().front().empty();
                    }
                }

                inline const usize row(void) const noexcept
                {
                    if constexpr (st == StoreType::Row)
                    {
                        return body().size();
                    }
                    else
                    {
                        return body().empty() ? 0_uz : body().front().size();
                    }
                }

                inline const usize column(void) const noexcept
                {
                    return header().size();
                }

            public:
                inline const HeaderViewType header(void) const noexcept
                {
                    return Trait::get_const_header(self());
                }

                inline const bool contains_header(const StringViewType header) const noexcept
                {
                    return header_index(header) != std::nullopt;
                }

                inline const std::optional<usize> header_index(const StringViewType header) const noexcept
                {
                    return Trait::get_column_index(header);
                }

                inline void set_header(const usize col, StringType header, const std::type_index type) noexcept
                {
                    Trait::set_header(self(), col, DataTableHeader{ header, type });
                }

                inline void set_header(const usize col, StringType header, std::initializer_list<std::type_index> types) noexcept
                {
                    Trait::set_header(self(), col, DataTableHeader{ header, std::set<std::type_index>{ std::move(types) } });
                }

                template<typename T>
                inline void set_header(const usize col, StringType header) noexcept
                {
                    set_header(col, header, ospf::TypeInfo<T>::index());
                }

                template<typename... Args>
                inline void set_header(const usize col, StringType header) noexcept
                {
                    set_header(col, header, { ospf::TypeInfo<Args>::index()... });
                }

                inline CLRefType<TableType> body(void) const noexcept
                {
                    return Trait::get_const_table(self());
                }

            public:
                inline CellWrapperType operator[](const std::array<usize, 2_uz> vector)
                {
                    return CellWrapperType{ Trait::get_table(self()), vector[0_uz], vector[1_uz] };
                }

                inline CLRefType<CellType> operator[](const std::array<usize, 2_uz> vector) const
                {
                    if constexpr (st == StoreType::Row)
                    {
                        return Trait::get_const_table(self())[vector[0_uz]][vector[1_uz]];
                    }
                    else
                    {
                        return Trait::get_const_table(self())[vector[1_uz]][vector[0_uz]];
                    }
                }

                inline CellWrapperType operator[](const std::pair<usize, StringViewType> vector)
                {
                    return CellWrapperType{ Trait::get_table(self()), vector.first, header_index(vector.second) };
                }

                inline CLRefType<CellType> operator[](const std::pair<usize, StringViewType> vector) const
                {
                    if constexpr (st == StoreType::Row)
                    {
                        return Trait::get_const_table(self())[vector.first][header_index(vector.second)];
                    }
                    else
                    {
                        return Trait::get_const_table(self())[header_index(vector.second)][vector.first];
                    }
                }

                inline RetType<RowViewType> operator[](const std::pair<usize, RangeFull> vector) const
                {
                    return row(vector.first);
                }

                inline RetType<ColumnViewType> operator[](const std::pair<RangeFull, usize> vector) const
                {
                    return column(vector.second);
                }

                inline RetType<ColumnViewType> operator[](const std::pair<RangeFull, StringViewType> vector) const
                {
                    return column(vector.second);
                }

            public:
                inline RetType<RowsViewType> rows(void) const noexcept
                {
                    return RowsViewType{ const_cast<Self&>(self()) };
                }

                inline RetType<ColumnsViewType> columns(void) const noexcept
                {
                    return ColumnsViewType{ const_cast<Self&>(self()) };
                }

                inline RetType<RowViewType> row(const usize i) const
                {
                    return Trait::get_row(self(), i);
                }

                inline RetType<ColumnViewType> column(const usize i) const
                {
                    return Trait::get_column(self(), i);
                }

                inline RetType<ColumnViewType> column(const StringViewType header) const
                {
                    return Trait::get_column(self(), header_index(header));
                }

            public:
                inline void clear_body(void)
                {
                    Trait::clear_table(self());
                }

                inline void clear(void)
                {
                    Trait::clear_header(self());
                    Trait::clear_table(self());
                }

                template<typename = void>
                    requires WithDefault<CellType>
                inline const usize insert_row(const usize pos)
                {
                    return insert_row(pos, DefaultValue<CellType>::value());
                }

                inline const usize insert_row(const usize pos, ArgCLRefType<CellType> value)
                {
                    Trait::insert_row(self(), pos, value);
                    return row + 1_uz;
                }

                template<typename U>
                    requires std::convertible_to<U, CellType>
                inline const usize insert_row(const usize pos, U&& value)
                {
                    return insert_row(pos, static_cast<CellType>(std::forward<U>(value)));
                }

                template<typename F>
                    requires requires (const F& fun, const usize j)
                    {
                        { fun(j) } -> DecaySameAs<CellType>;
                    }
                inline const usize insert_row(const usize pos, const F& constructor)
                {
                    return insert_row(pos, [&constructor](const usize i, const DataTableHeader<CharT>& _)
                        {
                            return constructor(i);
                        });
                }

                template<typename F>
                    requires RowConstructorType<F, CellType, CharT>
                inline const usize insert_row(const usize pos, const F& constructor)
                {
                    Trait::insert_row(self(), pos, RowConstructor{ constructor });
                    return pos + 1_uz;
                }

                template<typename = void>
                    requires WithDefault<CellType>
                inline RetType<RowIterType> insert_row(ArgCLRefType<RowIterType> pos)
                {
                    insert_row(static_cast<const usize>(pos));
                    return pos + 1_iz;
                }

                inline RetType<RowIterType> insert_row(ArgCLRefType<RowIterType> pos, ArgCLRefType<CellType> value)
                {
                    insert_row(static_cast<const usize>(pos), value);
                    return pos + 1_iz;
                }

                template<typename U>
                inline RetType<RowIterType> insert_row(ArgCLRefType<RowIterType> pos, U&& value)
                {
                    return insert_row(pos, CellType{ std::forward<U>(value) });
                }

                template<typename F>
                    requires requires (const F& fun, const usize j)
                    {
                        { fun(j) } -> DecaySameAs<CellType>;
                    }
                inline RetType<RowIterType> insert_row(ArgCLRefType<RowIterType> pos, const F& constructor)
                {
                    insert_row(static_cast<const usize>(pos), constructor);
                    return pos + 1_iz;
                }

                template<typename F>
                    requires RowConstructorType<F, CellType, CharT>
                inline RetType<RowIterType> insert_row(ArgCLRefType<RowIterType> pos, const F& constructor)
                {
                    insert_row(static_cast<const usize>(pos), constructor);
                    return pos + 1_iz;
                }

                inline const usize erase_row(const usize pos)
                {
                    Trait::erase_row(self(), pos);
                    return pos;
                }

                inline RetType<RowIterType> erase_row(ArgCLRefType<RowIterType> pos)
                {
                    erase_row(static_cast<const usize>(pos));
                    return pos;
                }

            private:
                struct Trait : public Self
                {
                    inline static LRefType<HeaderType> get_header(Self& self) noexcept
                    {
                        static const auto get_impl = &Self::OSPF_CRTP_FUNCTION(get_header);
                        return (self.*get_impl)();
                    }

                    inline void set_header(Self& self, const usize i, ArgRRefType<DataTableHeader<CharT>> header)
                    {
                        static const auto set_impl = &Self::OSPF_CRTP_FUNCTION(set_header);
                        return (self.*set_impl)(i, move<DataTableHeader<CharT>>(header));
                    }

                    inline static CLRefType<HeaderType> get_const_header(const Self& self) noexcept
                    {
                        static const auto get_impl = &Self::OSPF_CRTP_FUNCTION(get_const_header);
                        return (self.*get_impl)();
                    }

                    inline static const std::optional<usize> get_column_index(const Self& self, const StringViewType header) noexcept
                    {
                        static const auto get_impl = &Self::OSPF_CRTP_FUNCTION(get_column_index);
                        return (self.*get_impl)(header);
                    }

                    inline static LRefType<TableType> get_table(Self& self) noexcept
                    {
                        static const auto get_impl = &Self::OSPF_CRTP_FUNCTION(get_table);
                        return (self.*get_impl)();
                    }

                    inline static CLRefType<TableType> get_const_table(const Self& self) noexcept
                    {
                        static const auto get_impl = &Self::OSPF_CRTP_FUNCTION(get_const_table);
                        return (self.*get_impl)();
                    }

                    inline static RetType<RowViewType> get_row(const Self& self, const usize i)
                    {
                        static const auto get_impl = &Self::OSPF_CRTP_FUNCTION(get_row);
                        return (self.*get_impl)(i);
                    }

                    inline static RetType<ColumnViewType> get_column(const Self& self, const usize i)
                    {
                        static const auto get_impl = &Self::OSPF_CRTP_FUNCTION(get_column);
                        return (self.*get_impl)(i);
                    }

                    inline static void insert_row(Self& self, const usize pos, ArgCLRefType<CellType> value)
                    {
                        static const auto impl = &Self::OSPF_CRTP_FUNCTION(insert_row_by_value);
                        return (self.*impl)(pos, value);
                    }

                    inline static void insert_row(Self& self, const usize pos, const RowConstructor& constructor)
                    {
                        static const auto impl = &Self::OSPF_CRTP_FUNCTION(insert_row_by_constructor);
                        return (self.*impl)(pos, constructor);
                    }

                    inline static void erase_row(Self& self, const usize pos)
                    {
                        static const auto impl = &Self::OSPF_CRTP_FUNCTION(erase_row);
                        return (self.*impl)(pos);
                    }

                    inline static void clear_header(Self& self)
                    {
                        static const auto impl = &Self::OSPF_CRTP_FUNCTION(clear_header);
                        return (self.*impl)();
                    }

                    inline static void clear_table(Self& self)
                    {
                        static const auto impl = &Self::OSPF_CRTP_FUNCTION(clear_table);
                        return (self.*impl)();
                    }
                };
            };
        };
    };
};
