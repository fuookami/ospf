#pragma once

#include <ospf/ospf_base_api.hpp>
#include <ospf/basic_definition.hpp>
#include <ospf/literal_constant.hpp>

namespace ospf
{
    inline namespace uuid
    {
        static constexpr const usize uuid_length = 16_uz;

        using UUID = std::array<ubyte, uuid_length>;

        OSPF_BASE_API UUID uuid1(void) noexcept;
        OSPF_BASE_API UUID uuid3(const std::string_view name) noexcept;
        OSPF_BASE_API UUID uuid3(const UUID& nsp, const std::string_view name) noexcept;
        OSPF_BASE_API UUID uuid4(void) noexcept;
        OSPF_BASE_API UUID uuid5(const std::string_view name) noexcept;
        OSPF_BASE_API UUID uuid5(const UUID& nsp, const std::string_view name) noexcept;

        OSPF_BASE_API std::string to_string(const UUID& uuid) noexcept;
    };
};
