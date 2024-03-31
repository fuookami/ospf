#define BOOST_TEST_MODULE frontend_unit_test
#include <boost/test/included/unit_test.hpp>
#include <ospf/meta_programming/name_transfer/frontend.hpp>

BOOST_AUTO_TEST_CASE(camel_case_frontend_test)
{
    using namespace ospf::meta_programming;
    using namespace ospf::meta_programming::name_transfer;

    const Frontend<NamingSystem::CamelCase, char> frontend{};
    BOOST_ASSERT((frontend("bpp3dFaker", std::set<std::string_view>{ "bpp" }) == std::vector<std::string_view>{ "bpp", "3d", "Faker" }));
    BOOST_ASSERT((frontend("bpp3dFaker", std::set<std::string_view>{ "bpp", "bpp3d" }) == std::vector<std::string_view>{ "bpp3d", "Faker" }));
    BOOST_ASSERT((frontend("askBPP3DFaker", std::set<std::string_view>{ "bpp", "bpp3d" }) == std::vector<std::string_view>{ "ask", "bpp3d", "Faker" }));
    BOOST_ASSERT((frontend("askBpp3dFaker", std::set<std::string_view>{ "bpp", "bpp3d" }) == std::vector<std::string_view>{ "ask", "bpp3d", "Faker" }));
}
