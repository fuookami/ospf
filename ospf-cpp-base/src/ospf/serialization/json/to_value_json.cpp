#include <ospf/serialization/json/to_value.hpp>

namespace ospf::serialization::json
{
    rapidjson::Value from_bool(const bool value, rapidjson::Document& doc) noexcept
    {
        return rapidjson::Value{ value ? rapidjson::kTrueType : rapidjson::kFalseType };
    }

    rapidjson::Value from_u8(const u8 value, rapidjson::Document& doc) noexcept
    {
        return rapidjson::Value{ static_cast<u32>(value) };
    }

    rapidjson::Value from_i8(const i8 value, rapidjson::Document& doc) noexcept
    {
        return rapidjson::Value{ static_cast<i32>(value) };
    }

    rapidjson::Value from_u16(const u16 value, rapidjson::Document& doc) noexcept
    {
        return rapidjson::Value{ static_cast<u32>(value) };
    }

    rapidjson::Value from_i16(const i16 value, rapidjson::Document& doc) noexcept
    {
        return rapidjson::Value{ static_cast<i32>(value) };
    }

    rapidjson::Value from_u32(const u32 value, rapidjson::Document& doc) noexcept
    {
        return rapidjson::Value{ value };
    }

    rapidjson::Value from_i32(const i32 value, rapidjson::Document& doc) noexcept
    {
        return rapidjson::Value{ value };
    }

    rapidjson::Value from_u64(const u64 value, rapidjson::Document& doc) noexcept
    {
        return rapidjson::Value{ value };
    }

    rapidjson::Value from_i64(const i64 value, rapidjson::Document& doc) noexcept
    {
        return rapidjson::Value{ value };
    }

    rapidjson::Value from_f32(const f32 value, rapidjson::Document& doc) noexcept
    {
        return rapidjson::Value{ value };
    }

    rapidjson::Value from_f64(const f64 value, rapidjson::Document& doc) noexcept
    {
        return rapidjson::Value{ value };
    }

    rapidjson::Value from_string(const std::string& value, rapidjson::Document& doc) noexcept
    {
        return rapidjson::Value{ value.c_str(), doc.GetAllocator() };
    }

    rapidjson::Value from_string_view(const std::string_view value, rapidjson::Document& doc) noexcept
    {
        return rapidjson::Value{ value.data(), doc.GetAllocator() };
    }
};
