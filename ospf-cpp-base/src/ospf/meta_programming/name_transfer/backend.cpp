#include <ospf/meta_programming/name_transfer/backend.hpp>

namespace ospf::meta_programming::name_transfer
{
    template struct Backend<NamingSystem::SnakeCase, char>;
    template struct Backend<NamingSystem::SnakeCase, wchar>;

    template struct Backend<NamingSystem::KebabCase, char>;
    template struct Backend<NamingSystem::KebabCase, wchar>;

    template struct Backend<NamingSystem::CamelCase, char>;
    template struct Backend<NamingSystem::CamelCase, wchar>;

    template struct Backend<NamingSystem::PascalCase, char>;
    template struct Backend<NamingSystem::PascalCase, wchar>;

    template struct Backend<NamingSystem::UpperSnakeCase, char>;
    template struct Backend<NamingSystem::UpperSnakeCase, wchar>;
};
