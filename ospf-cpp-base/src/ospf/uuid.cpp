#include <ospf/uuid.hpp>
#include <ospf/random.hpp>
#include <boost/uuid/uuid.hpp>
#include <boost/uuid/name_generator.hpp>
#include <boost/uuid/name_generator_md5.hpp>
#include <boost/uuid/name_generator_sha1.hpp>
#include <boost/uuid/random_generator.hpp>

#ifdef _WIN32
#include <objbase.h>
#else
#include <uuid/uuid.h>
#endif

namespace ospf::uuid
{
    namespace detail
    {
        inline const boost::uuids::uuid& wrap(const UUID& raw) noexcept
        {
            return *reinterpret_cast<const boost::uuids::uuid*>(raw.data());
        }

        template<typename U>
        inline UUID dump(const U& raw) noexcept
        {
            UUID ret{ static_cast<std::byte>(0) };
            std::copy(reinterpret_cast<const std::byte*>(&raw), reinterpret_cast<const std::byte*>(&raw) + uuid_length, ret.begin());
            return ret;
        }
    };

    UUID uuid1(void) noexcept
    {
#ifdef _WIN32
        GUID uuid;
        CoCreateGuid(&uuid);
#else
        UUID uuid;
        uuid_generate(reinterpret_cast<unsigned char*>(&uuid));
#endif

        return detail::dump(uuid);
    }

    UUID uuid3(const std::string_view name) noexcept
    {
        static auto raw_nsp = uuid1();
        static auto nsp = detail::wrap(raw_nsp);
        static boost::uuids::name_generator_md5 uuid_gen{ nsp };

        return detail::dump(uuid_gen(name.data()));
    }

    UUID uuid3(const UUID& raw_nsp, const std::string_view name) noexcept
    {
        const auto nsp = detail::wrap(raw_nsp);
        boost::uuids::name_generator_md5 uuid_gen{ nsp };

        return detail::dump(uuid_gen(name.data()));
    }

    UUID uuid4(void) noexcept
    {
        static auto random_gen = random_generator<u64>();
        static boost::uuids::basic_random_generator<decltype(random_gen)> uuid_gen{ random_gen };

        return detail::dump(uuid_gen());
    }

    UUID uuid5(const std::string_view name) noexcept
    {
        static auto raw_nsp = uuid1();
        static auto nsp = detail::wrap(raw_nsp);
        static boost::uuids::name_generator_sha1 uuid_gen{ nsp };

        return detail::dump(uuid_gen(name.data()));
    }

    UUID uuid5(const UUID& raw_nsp, const std::string_view name) noexcept
    {
        const auto nsp = detail::wrap(raw_nsp);
        boost::uuids::name_generator_sha1 uuid_gen{ nsp };

        return detail::dump(uuid_gen(name.data()));
    }

    std::string to_string(const UUID& uuid) noexcept
    {
        static const auto splitor_positions = std::array{ 4_uz, 6_uz, 8_uz, 10_uz };

        std::ostringstream sout;
        for (auto i{ 0_uz }, j{ 0_uz }; i != uuid_length; ++i)
        {
            if (j != splitor_positions.size() && i == splitor_positions[j])
            {
                sout << '-';
                ++j;
            }
            sout << std::hex << static_cast<i32>(uuid[i]);
        }
        return sout.str();
    }
};
