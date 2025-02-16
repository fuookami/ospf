#include <ospf/serialization/csv/from_value.hpp>
#include <boost/algorithm/string.hpp>
#include <boost/lexical_cast.hpp>

namespace ospf::serialization::csv
{
    std::optional<bool> to_bool(const std::string_view value) noexcept
    {
        const auto int_value = to_u8(value);
        if (int_value.has_value())
        {
            return static_cast<bool>(int_value);
        }

        std::string string{ value };
        boost::to_lower(string);
        if (string == "true")
        {
            return true;
        }
        else if (string == "false")
        {
            return false;
        }
        else
        {
            return std::nullopt;
        }
    }

    std::optional<u8> to_u8(const std::string_view value) noexcept
    {
        try
        {
            return boost::lexical_cast<u8>(value);
        }
        catch (...)
        {
            return std::nullopt;
        }
    }

    std::optional<i8> to_i8(const std::string_view value) noexcept
    {
        try
        {
            return boost::lexical_cast<i8>(value);
        }
        catch (...)
        {
            return std::nullopt;
        }
    }

    std::optional<u16> to_u16(const std::string_view value) noexcept
    {
        try
        {
            return boost::lexical_cast<u16>(value);
        }
        catch (...)
        {
            return std::nullopt;
        }
    }

    std::optional<i16> to_i16(const std::string_view value) noexcept
    {
        try
        {
            return boost::lexical_cast<i16>(value);
        }
        catch (...)
        {
            return std::nullopt;
        }
    }

    std::optional<i32> to_i32(const std::string_view value) noexcept
    {
        try
        {
            return boost::lexical_cast<i32>(value);
        }
        catch (...)
        {
            return std::nullopt;
        }
    }

    std::optional<u32> to_u32(const std::string_view value) noexcept
    {
        try
        {
            return boost::lexical_cast<u32>(value);
        }
        catch (...)
        {
            return std::nullopt;
        }
    }

    std::optional<i64> to_i64(const std::string_view value) noexcept
    {
        try
        {
            return boost::lexical_cast<i64>(value);
        }
        catch (...)
        {
            return std::nullopt;
        }
    }

    std::optional<u64> to_u64(const std::string_view value) noexcept
    {
        try
        {
            return boost::lexical_cast<u64>(value);
        }
        catch (...)
        {
            return std::nullopt;
        }
    }

    std::optional<f32> to_f32(const std::string_view value) noexcept
    {
        try
        {
            return boost::lexical_cast<f32>(value);
        }
        catch (...)
        {
            return std::nullopt;
        }
    }

    std::optional<f64> to_f64(const std::string_view value) noexcept
    {
        try
        {
            return boost::lexical_cast<f64>(value);
        }
        catch (...)
        {
            return std::nullopt;
        }
    }
};
