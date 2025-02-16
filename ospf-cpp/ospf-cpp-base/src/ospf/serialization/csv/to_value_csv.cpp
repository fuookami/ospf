#include <ospf/serialization/csv/to_value.hpp>

namespace ospf::serialization::csv
{
    std::string from_bool(const bool value) noexcept
    {
        return value ? "true" : "false";
    }

    std::string from_u8(const u8 value) noexcept
    {
        return std::to_string(value);
    }

    std::string from_i8(const i8 value) noexcept
    {
        return std::to_string(value);
    }

    std::string from_u16(const u16 value) noexcept
    {
        return std::to_string(value);
    }

    std::string from_i16(const i16 value) noexcept
    {
        return std::to_string(value);
    }

    std::string from_u32(const u32 value) noexcept
    {
        return std::to_string(value);
    }

    std::string from_i32(const i32 value) noexcept
    {
        return std::to_string(value);
    }

    std::string from_u64(const u64 value) noexcept
    {
        return std::to_string(value);
    }

    std::string from_i64(const i64 value) noexcept
    {
        return std::to_string(value);
    }

    std::string from_f32(const f32 value) noexcept
    {
        return std::to_string(value);
    }

    std::string from_f64(const f64 value) noexcept
    {
        return std::to_string(value);
    }
};
