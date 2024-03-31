#pragma once

#include <ospf/functional/result.hpp>
#include <ospf/string/format.hpp>
#include <rapidjson/document.h>
#include <rapidjson/istreamwrapper.h>
#include <rapidjson/ostreamwrapper.h>
#include <rapidjson/writer.h>
#include <rapidjson/error/en.h>
#include <istream>
#include <ostream>

namespace ospf
{
    inline namespace serialization
    {
        namespace json
        {
            template<CharType CharT>
            inline Try<> write(std::basic_ostream<CharT>& os, const Document<CharT>& doc) noexcept
            {
                rapidjson::BasicOStreamWrapper<OriginType<decltype(os)>> osw{ os };
                rapidjson::Writer<OriginType<decltype(osw)>> writer{ osw };

                if (!doc.Accept(writer))
                {
                    return ospf::OSPFError{ ospf::OSPFErrCode::SerializationFail };
                }
                return succeed;
            }

            template<CharType CharT>
            inline Result<Document<CharT>> read(std::basic_istream<CharT>& is) noexcept
            {
                Document<CharT> doc;
                rapidjson::BasicIStreamWrapper<OriginType<decltype(is)>> isw{is};
                rapidjson::ParseResult ok = doc.ParseStream(isw);
                if (!ok)
                {
                    if constexpr (DecaySameAs<CharT, char>)
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("{} at {}", rapidjson::GetParseError_En(ok.Code()), ok.Offset()) };
                    }
                    else if constexpr (DecaySameAs<CharT, wchar>)
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format(L"{} at {}", rapidjson::GetParseError_En(ok.Code()), ok.Offset()) };
                    }
                    //else
                    //{
                    //    static_assert(false, "unsupported character type");
                    //}
                }
                else
                {
                    return std::move(doc);
                }
            }
        };
    };
};
