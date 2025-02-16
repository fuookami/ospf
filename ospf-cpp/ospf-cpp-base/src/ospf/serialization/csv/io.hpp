#pragma once

#include <ospf/functional/result.hpp>
#include <ospf/serialization/csv/concepts.hpp>
#include <ospf/serialization/csv/table.hpp>

namespace ospf
{
    inline namespace serialization
    {
        namespace csv
        {
            template<typename T, CharType CharT>
            inline Try<> write(std::basic_ostream<CharT>& os, const T& table, const std::basic_string_view<CharT> seperator = CharTrait<CharT>::default_seperator) noexcept
            {
                if (table.header().empty())
                {
                    return OSPFError{ OSPFErrCode::DataEmpty };
                }

                const auto column = table.column();
                for (usize j{ 0_uz }; j != column; ++j)
                {
                    CharTrait<CharT>::write(os, table.header()[j].name(), seperator);
                    if (j != (column - 1_uz))
                    {
                        os << seperator;
                    }
                }
                os << CharTrait<CharT>::line_breaker;

                for (const auto& row : table.rows())
                {
                    for (usize j{ 0_uz }; j != column; ++j)
                    {
                        CharTrait<CharT>::write(os, row[j], seperator);
                        if (j != (column - 1_uz))
                        {
                            os << seperator;
                        }
                    }
                }
                os << CharTrait<CharT>::line_breaker;

                return ospf::succeed;
            }

            template<CharType CharT>
            inline Result<CSVTable<CharT>> read(std::basic_istream<CharT>& is, const std::basic_string_view<CharT> seperator = CharTrait<CharT>::default_seperator) noexcept
            {
                std::basic_string<CharT> line;
                if (!std::getline(is, line))
                {
                    is.setstate(std::ios_base::failbit);
                    return OSPFError{ OSPFErrCode::DataEmpty };
                }

                const auto regex_matcher = CharTrait<CharT>::catch_regex(seperator);
                std::vector<std::basic_string<CharT>> header{};
                auto headers = regex_catch(line, regex_matcher);
                for (usize j{ 0_uz }; j != headers.size(); ++j)
                {
                    header.push_back(CharTrait<CharT>::extract(headers[j], seperator));
                }

                CSVTable table{ std::move(header) };
                while (std::getline(is, line))
                {
                    if (line.empty())
                    {
                        continue;
                    }

                    auto this_row = regex_catch(line, regex_matcher);
                    table.insert_row(table.row(), [&this_row, seperator](const usize j)
                        {
                            return CharTrait<CharT>::extract(this_row[j], seperator);
                        });
                }
                return std::move(table);
            }

            template<usize col, CharType CharT>
            inline Result<ORMCSVTable<col, CharT>> read(std::basic_istream<CharT>& is, const std::basic_string_view<CharT> seperator = CharTrait<CharT>::default_seperator) noexcept
            {
                std::basic_string<CharT> line;
                if (!std::getline(is, line))
                {
                    is.setstate(std::ios_base::failbit);
                    return OSPFError{ OSPFErrCode::DataEmpty };
                }

                const auto regex_matcher = CharTrait<CharT>::catch_regex(seperator);
                std::array<std::basic_string<CharT>, col> header{};
                auto headers = regex_catch(line, regex_matcher);
                if (headers.size() != col)
                {
                    return OSPFError{ OSPFErrCode::DeserializationFail, "unmatched header size" };
                }
                for (usize j{ 0_uz }; j != col; ++j)
                {
                    header[j] = CharTrait<CharT>::extract(headers[j], seperator);
                }

                ORMCSVTable<col> table{ header };
                while (std::getline(is, line))
                {
                    if (line.empty())
                    {
                        continue;
                    }

                    auto this_row = regex_catch(line, regex_matcher);
                    table.insert_row(table.row(), [&this_row, seperator](const usize j)
                        {
                            return CharTrait<CharT>::extract(this_row[j], seperator);
                        });
                }
                
                return std::move(table);
            }
        };
    };
};
