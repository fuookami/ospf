#include <ospf/meta_programming/name_transfer/frontend.hpp>

namespace ospf::meta_programming::name_transfer
{
    template struct Frontend<NamingSystem::SnakeCase, char>;
    template struct Frontend<NamingSystem::SnakeCase, wchar>;

    template struct Frontend<NamingSystem::UpperSnakeCase, char>;
    template struct Frontend<NamingSystem::UpperSnakeCase, wchar>;

    template struct Frontend<NamingSystem::KebabCase, char>;
    template struct Frontend<NamingSystem::KebabCase, wchar>;

    template struct Frontend<NamingSystem::CamelCase, char>;
    template struct Frontend<NamingSystem::CamelCase, wchar>;

    template struct Frontend<NamingSystem::PascalCase, char>;
    template struct Frontend<NamingSystem::PascalCase, wchar>;
};
