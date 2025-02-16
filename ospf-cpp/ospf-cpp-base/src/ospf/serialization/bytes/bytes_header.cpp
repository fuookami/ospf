#include <ospf/serialization/bytes/header.hpp>

namespace ospf::serialization::bytes
{
    const bool same(const Fields& lhs, const Fields& rhs, const bool recursive) noexcept
    {
        for (const auto& [key, _] : rhs)
        {
            if (!lhs.contains(key))
            {
                return false;
            }
        }
        for (const auto& [key, field] : lhs)
        {
            if (!rhs.contains(key))
            {
                return false;
            }
            if (recursive && !same(field->fields(), rhs.at(key)->fields(), true))
            {
                return false;
            }
            if (!recursive && *field != *rhs.at(key))
            {
                return false;
            }
        }
        return true;
    }

    const bool SubHeader::operator==(const SubHeader& rhs) const noexcept
    {
        return _id == rhs._id
            && _tag == rhs._tag
            && same(_fields, rhs._fields, false);
    }

    const bool SubHeader::operator!=(const SubHeader& rhs) const noexcept
    {
        return _id != rhs._id
            || _tag != rhs._tag
            || !same(_fields, rhs._fields, false);
    }

    const usize ToBytesValue<Header>::size(const Header& header) const noexcept
    {

        usize ret{ 0_uz };
        ret += 1_uz;
        ret += 8_uz;
        {
            static const ToBytesValue<std::span<const usize>> serializer{};
            ret += serializer.size(header.field_segement());
            ret += serializer.size(header.segement());
        }
        ret += address_length;
        for (const auto& sub_header : header.sub_headers())
        {
            ret += size(*sub_header);
        }
        ret += size(header.fields());
        return ret;
    }

    const usize ToBytesValue<Header>::size(const SubHeader& sub_header) const noexcept
    {
        usize ret{ 0_uz };
        {
            static const ToBytesValue<HeaderTag> serializer{};
            ret += serializer.size(sub_header.tag());
        }
        ret += 8_uz;
        ret += size(sub_header.fields());
        return ret;
    }

    const usize ToBytesValue<Header>::size(const Fields& fields) const noexcept
    {
        usize ret{ 8_uz };
        for (const auto& [key, _] : fields)
        {
            static const ToBytesValue<std::string_view> serializer{};
            ret += serializer.size(key);
            ret += 8_uz;
        }
        return ret;
    }
};
