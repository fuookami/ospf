#include <ospf/serialization/json/from_value.hpp>

namespace ospf::serialization::json
{
    std::optional<bool> to_bool(const rapidjson::Value& json) noexcept
    {
        if (!json.IsBool())
        {
            return std::nullopt;
        }
        return json.GetBool();
    }

    std::optional<u8> to_u8(const rapidjson::Value& json) noexcept
    {
        if (!json.IsUint())
        {
            return std::nullopt;
        }
        return static_cast<u8>(json.GetUint());
    }

    std::optional<i8> to_i8(const rapidjson::Value& json) noexcept
    {
        if (!json.IsInt())
        {
            return std::nullopt;
        }
        return static_cast<i8>(json.GetInt());
    }

    std::optional<u16> to_u16(const rapidjson::Value& json) noexcept
    {
        if (!json.IsUint())
        {
            return std::nullopt;
        }
        return static_cast<u16>(json.GetUint());
    }

    std::optional<i16> to_i16(const rapidjson::Value& json) noexcept
    {
        if (!json.IsInt())
        {
            return std::nullopt;
        }
        return static_cast<i16>(json.GetInt());
    }

    std::optional<u32> to_u32(const rapidjson::Value& json) noexcept
    {
        if (!json.IsUint())
        {
            return std::nullopt;
        }
        return json.GetUint();
    }

    std::optional<i32> to_i32(const rapidjson::Value& json) noexcept
    {
        if (!json.IsInt())
        {
            return std::nullopt;
        }
        return json.GetInt();
    }

    std::optional<u64> to_u64(const rapidjson::Value& json) noexcept
    {
        if (!json.IsUint64())
        {
            return std::nullopt;
        }
        return json.GetUint64();
    }

    std::optional<i64> to_i64(const rapidjson::Value& json) noexcept
    {
        if (!json.IsInt64())
        {
            return std::nullopt;
        }
        return json.GetInt64();
    }

    std::optional<f32> to_f32(const rapidjson::Value& json) noexcept
    {
        if (!json.IsLosslessFloat())
        {
            return std::nullopt;
        }
        return json.GetFloat();
    }

    std::optional<f64> to_f64(const rapidjson::Value& json) noexcept
    {
        if (!json.IsLosslessDouble())
        {
            return std::nullopt;
        }
        return json.GetDouble();
    }

    std::optional<std::string> to_string(const rapidjson::Value& json) noexcept
    {
        if (!json.IsString())
        {
            return std::nullopt;
        }
        return std::string{ json.GetString() };
    }
};
