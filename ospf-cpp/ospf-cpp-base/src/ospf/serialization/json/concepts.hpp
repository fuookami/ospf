#pragma once

#include <ospf/exception.hpp>
#include <rapidjson/document.h>
#include <rapidjson/ostreamwrapper.h>
#include <rapidjson/writer.h>
#include <format>
#include <functional>
#include <string_view>
#include <sstream>

namespace ospf
{
    inline namespace serialization
    {
        namespace json
        {
            template<CharType CharT>
            using Json = rapidjson::GenericValue<rapidjson::UTF8<CharT>>;

            template<CharType CharT>
            using Object = rapidjson::GenericObject<false, Json<CharT>>;

            template<CharType CharT>
            using ObjectView = rapidjson::GenericObject<true, Json<CharT>>;

            template<CharType CharT>
            using Array = rapidjson::GenericArray<false, Json<CharT>>;

            template<CharType CharT>
            using ArrayView = rapidjson::GenericArray<true, Json<CharT>>;

            template<CharType CharT>
            using Document = rapidjson::GenericDocument<rapidjson::UTF8<CharT>>;

            template<CharType CharT>
            using NameTransfer = std::function<const std::basic_string_view<CharT>(const std::basic_string_view<CharT>)>;
        };
    };
};

namespace std
{
    template<ospf::CharType CharT>
    struct formatter<ospf::serialization::json::Json<CharT>, CharT>
        : public formatter<basic_string_view<CharT>, CharT>
    {
        template<typename FormatContext>
        inline constexpr decltype(auto) format(const ospf::serialization::json::Json<CharT>& value, FormatContext& fc) const
        {
            basic_ostringstream<CharT> sout;
            rapidjson::BasicOStreamWrapper<decltype(sout)> osw{sout};
            rapidjson::Writer<decltype(osw)> writer{ osw };

            if (!value.Accept(writer))
            {
                throw ospf::OSPFException{ ospf::OSPFErrCode::SerializationFail };
            }

            static const auto _formatter = formatter<basic_string_view<CharT>, CharT>{};
            return _formatter.format(sout.str(), fc);
        }
    };
};
